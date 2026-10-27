use aide::axum::routing::{delete_with, get_with, post_with, put_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use aide::OperationOutput;
use axum::body::Bytes;
use axum::extract::{Multipart, Path};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models;
use crate::request_state::RequestState;

use super::account_status::AccountStatusDto;
use super::accounts::CoinAmountDto;

const SUPPORTED_IMAGE_TYPES: [&str; 6] = [
    "image/png",
    "image/jpeg",
    "image/jpg",
    "image/webp",
    "image/svg",
    "image/svg+xml",
];

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/product/:id/image",
            get_with(get_product_image, get_product_image_docs)
                .put_with(upload_product_image, upload_product_image_docs)
                .delete_with(delete_product_image, delete_product_image_docs),
        )
        .api_route(
            "/product/:id",
            get_with(get_product, get_product_docs)
                .put_with(update_product, update_product_docs)
                .delete_with(delete_product, delete_product_docs),
        )
        .api_route(
            "/products",
            get_with(list_products, list_products_docs)
                .post_with(create_product, create_product_docs),
        )
        .api_route(
            "/products/by-barcode/:barcode",
            get_with(get_product_by_barcode, get_product_by_barcode_docs),
        )
        .api_route(
            "/products/by-barcode/:barcode/conflicts",
            get_with(get_barcode_conflicts, get_barcode_conflicts_docs),
        )
        .api_route(
            "/product/:id/barcodes",
            post_with(add_product_barcode, add_product_barcode_docs),
        )
        .api_route(
            "/product/:id/barcode/:barcode_id",
            delete_with(delete_product_barcode, delete_product_barcode_docs),
        )
        .api_route(
            "/product/:id/sub-products",
            post_with(add_sub_product, add_sub_product_docs),
        )
        .api_route(
            "/product/:id/sub-product/:sub_product_id",
            put_with(update_sub_product, update_sub_product_docs),
        )
        .api_route(
            "/products/inventory",
            put_with(update_products_inventory, update_products_inventory_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ProductStatusPriceDto {
    pub status: AccountStatusDto,
    pub price: CoinAmountDto,
    pub bonus: CoinAmountDto,
}

impl From<&models::ProductStatusPrice> for ProductStatusPriceDto {
    fn from(value: &models::ProductStatusPrice) -> Self {
        Self {
            status: (&value.status).into(),
            price: (&value.price).into(),
            bonus: (&value.bonus).into(),
        }
    }
}

/// One barcode of a product: a purchasing variant with its own container size.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ProductBarcodeDto {
    pub id: u64,
    pub code: String,
    /// Single units this code stands for (crate of 20 => 20).
    pub container_size: i32,
    /// Free text shown next to the code, e.g. "Kasten 20x0,5l".
    pub label: String,
    /// Sub product this code belongs to; `null` means the product itself.
    pub sub_product_id: Option<u64>,
}

impl From<&models::ProductBarcode> for ProductBarcodeDto {
    fn from(value: &models::ProductBarcode) -> Self {
        Self {
            id: value.id,
            code: value.code.to_owned(),
            container_size: value.container_size,
            label: value.label.to_owned(),
            sub_product_id: value.sub_product_id,
        }
    }
}

/// A purchasing variant that is tracked on its own (name, codes, purchase tax, price history)
/// but sold as the parent product.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SubProductDto {
    pub id: u64,
    pub name: String,
    pub purchase_tax: i32,
    /// Inactive sub products keep their history but are no longer offered.
    pub active: bool,
    /// Listed below the product on every print list (snack list) the product is on.
    pub show_on_print_list: bool,
    /// Target stock in single units (inventory is counted per sub product).
    pub target_quantity: i32,
}

impl From<&models::SubProduct> for SubProductDto {
    fn from(value: &models::SubProduct) -> Self {
        Self {
            id: value.id,
            name: value.name.to_owned(),
            purchase_tax: value.purchase_tax,
            active: value.active,
            show_on_print_list: value.show_on_print_list,
            target_quantity: value.target_quantity,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ProductDto {
    pub id: u64,
    pub name: String,
    pub price: CoinAmountDto,
    pub bonus: CoinAmountDto,
    pub purchase_tax: i32,
    pub nickname: Option<String>,
    /// Primary (oldest) barcode of the product. Kept for clients that only know one code;
    /// `barcodes` is the full list.
    pub barcode: Option<String>,
    pub barcodes: Vec<ProductBarcodeDto>,
    pub sub_products: Vec<SubProductDto>,
    pub category: String,
    pub print_lists: Vec<String>,
    pub tags: Vec<String>,
    pub status_prices: Vec<ProductStatusPriceDto>,
    pub stocked: bool,
    pub target_quantity: i32,
}

impl From<&models::Product> for ProductDto {
    fn from(value: &models::Product) -> Self {
        Self {
            id: value.id.to_owned(),
            name: value.name.to_owned(),
            price: (&value.price).into(),
            bonus: (&value.bonus).into(),
            purchase_tax: value.purchase_tax,
            nickname: value.nickname.to_owned(),
            barcode: value.primary_barcode().map(|b| b.code.to_owned()),
            barcodes: value.barcodes.iter().map(ProductBarcodeDto::from).collect(),
            sub_products: value.sub_products.iter().map(SubProductDto::from).collect(),
            category: value.category.to_owned(),
            print_lists: value.print_lists.to_owned(),
            tags: value.tags.to_owned(),
            status_prices: value
                .status_prices
                .iter()
                .map(ProductStatusPriceDto::from)
                .collect(),
            stocked: value.stocked,
            target_quantity: value.target_quantity,
        }
    }
}

pub async fn list_products(mut state: RequestState) -> ServiceResult<Json<Vec<ProductDto>>> {
    let products = state.db.get_all_products().await?;
    Ok(Json(products.iter().map(|p| p.into()).collect()))
}

fn list_products_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all products.")
        .tag("products")
        .response::<200, Json<Vec<ProductDto>>>()
}

pub async fn get_product(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<ProductDto>> {
    let product = state.db.get_product_by_id(id).await?;

    if let Some(product) = product {
        return Ok(Json(ProductDto::from(&product)));
    }

    Err(ServiceError::NotFound)
}

fn get_product_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get a product by id.")
        .tag("products")
        .response::<200, Json<ProductDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested product does not exist!"))
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SaveProductStatusPriceDto {
    pub status_id: u64,
    pub price: CoinAmountDto,
    pub bonus: CoinAmountDto,
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SaveProductBarcodeDto {
    pub code: String,
    #[serde(default = "default_container_size")]
    pub container_size: i32,
    #[serde(default)]
    pub label: String,
    /// Existing sub product of this product the code belongs to.
    #[serde(default)]
    pub sub_product_id: Option<u64>,
    /// `key` of an entry in `sub_products` of the same save request. Lets a code reference a
    /// sub product that is created by that request. Takes precedence over `sub_product_id`.
    #[serde(default)]
    pub sub_product_key: Option<String>,
}

fn default_container_size() -> i32 {
    1
}

fn default_true() -> bool {
    true
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SaveSubProductDto {
    /// Existing sub product to update; absent creates a new one.
    #[serde(default)]
    pub id: Option<u64>,
    /// Client chosen key that barcodes of the same request can reference via
    /// `sub_product_key`.
    #[serde(default)]
    pub key: Option<String>,
    pub name: String,
    /// Defaults to the purchase tax of the parent product.
    #[serde(default)]
    pub purchase_tax: Option<i32>,
    #[serde(default = "default_true")]
    pub active: bool,
    /// Defaults to false.
    #[serde(default)]
    pub show_on_print_list: bool,
    /// Defaults to 0.
    #[serde(default)]
    pub target_quantity: i32,
}

impl SaveSubProductDto {
    fn to_model(&self, parent_purchase_tax: i32) -> models::SubProduct {
        models::SubProduct {
            id: self.id.unwrap_or(0),
            name: self.name.to_owned(),
            purchase_tax: self.purchase_tax.unwrap_or(parent_purchase_tax),
            active: self.active,
            show_on_print_list: self.show_on_print_list,
            target_quantity: self.target_quantity,
        }
    }
}

/// Checks a sub product reference of a purchase or shopping list item: it needs a product and
/// must be one of that product's sub products.
pub(super) fn resolve_sub_product(
    product: Option<&models::Product>,
    sub_product_id: Option<u64>,
) -> ServiceResult<Option<u64>> {
    let Some(sub_product_id) = sub_product_id else {
        return Ok(None);
    };
    match product {
        Some(product) if product.has_sub_product(sub_product_id) => Ok(Some(sub_product_id)),
        _ => Err(ServiceError::BadRequest(
            "sub product does not belong to the product".to_owned(),
        )),
    }
}

/// Turns the barcodes of a save request into models, resolving `sub_product_key` against the
/// sub products stored by the same request (`stored` is in the order of `sub_products`).
fn link_barcodes(
    barcodes: &[SaveProductBarcodeDto],
    sub_products: Option<&[SaveSubProductDto]>,
    stored: &[models::SubProduct],
) -> ServiceResult<Vec<models::ProductBarcode>> {
    barcodes
        .iter()
        .map(|barcode| {
            let sub_product_id = match barcode.sub_product_key.as_deref() {
                Some(key) => {
                    let index = sub_products
                        .unwrap_or_default()
                        .iter()
                        .position(|s| s.key.as_deref() == Some(key))
                        .ok_or_else(|| {
                            ServiceError::BadRequest(format!("unknown sub product key '{key}'"))
                        })?;
                    Some(stored[index].id)
                }
                None => barcode.sub_product_id,
            };
            Ok(models::ProductBarcode {
                id: 0,
                code: barcode.code.to_owned(),
                container_size: barcode.container_size,
                label: barcode.label.to_owned(),
                sub_product_id,
            })
        })
        .collect()
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SaveProductDto {
    pub name: String,
    pub price: CoinAmountDto,
    pub bonus: CoinAmountDto,
    pub purchase_tax: i32,
    pub nickname: Option<String>,
    /// Legacy single barcode. Only used when `barcodes` is absent, so a client that does not
    /// know about multiple barcodes cannot drop the ones it never saw.
    pub barcode: Option<String>,
    /// Full list of barcodes. When present it replaces all barcodes of the product.
    pub barcodes: Option<Vec<SaveProductBarcodeDto>>,
    /// Full list of sub products. When present it replaces the sub products of the product:
    /// entries with an `id` are updated, the others created. Sub products missing from the list
    /// are deleted, or only deactivated if something was already bought as them.
    #[serde(default)]
    pub sub_products: Option<Vec<SaveSubProductDto>>,
    pub category: String,
    pub print_lists: Vec<String>,
    pub tags: Vec<String>,
    pub status_prices: Vec<SaveProductStatusPriceDto>,
    #[serde(default)]
    pub stocked: bool,
    #[serde(default)]
    pub target_quantity: i32,
}

/// The legacy single `barcode` field as a barcode list, for create requests.
fn legacy_barcodes(barcode: Option<&str>) -> Vec<models::ProductBarcode> {
    barcode
        .iter()
        .filter(|code| !code.trim().is_empty())
        .map(|code| models::ProductBarcode {
            id: 0,
            code: code.to_string(),
            container_size: 1,
            label: String::new(),
            sub_product_id: None,
        })
        .collect()
}

/// The parts of a save request that are stored after the product row itself.
struct ProductChildren {
    /// Legacy single barcode (create only).
    barcode: Option<String>,
    barcodes: Option<Vec<SaveProductBarcodeDto>>,
    sub_products: Option<Vec<SaveSubProductDto>>,
}

/// Stores the product row, then its sub products, then its barcodes (which may reference sub
/// products created by the same request), and returns the product as stored.
///
/// `barcodes` is authoritative and replaces the whole list. `None` means the request does not
/// know about multiple barcodes: on create the legacy single `barcode` field is used, on update
/// the existing barcodes are left alone, so an old client that echoes `barcode` back cannot drop
/// the variants it never saw. `sub_products` works the same way (absent = untouched).
async fn save_product(
    state: &mut RequestState,
    mut product: models::Product,
    children: ProductChildren,
) -> ServiceResult<models::Product> {
    let is_new = product.id == 0;
    if is_new {
        // Barcodes may point at sub products that only exist once the product does.
        product.barcodes = Vec::new();
    }
    let product = state.db.store_product(product).await?;

    let stored_sub_products = match children.sub_products.as_deref() {
        Some(sub_products) => {
            let models: Vec<models::SubProduct> = sub_products
                .iter()
                .map(|s| s.to_model(product.purchase_tax))
                .collect();
            state.db.store_sub_products(product.id, &models).await?
        }
        None => Vec::new(),
    };

    let barcodes = match children.barcodes.as_deref() {
        Some(barcodes) => Some(link_barcodes(
            barcodes,
            children.sub_products.as_deref(),
            &stored_sub_products,
        )?),
        None if is_new => Some(legacy_barcodes(children.barcode.as_deref())),
        None => None,
    };
    if let Some(barcodes) = barcodes {
        let product_id = i64::try_from(product.id).expect("id is always less than 2**63");
        state
            .db
            .replace_product_barcodes(product_id, &barcodes)
            .await?;
    }

    state
        .db
        .get_product_by_id(product.id)
        .await?
        .ok_or(ServiceError::NotFound)
}

async fn create_product(
    mut state: RequestState,
    form: Json<SaveProductDto>,
) -> ServiceResult<Json<ProductDto>> {
    state.session_require_purchaser_or_admin()?;

    let form = form.0;

    let status_prices = resolve_status_prices(&mut state, &form.status_prices).await?;
    let children = ProductChildren {
        barcode: form.barcode,
        barcodes: form.barcodes,
        sub_products: form.sub_products,
    };

    let product = models::Product {
        id: 0,
        name: form.name,
        price: form.price.into(),
        bonus: form.bonus.into(),
        purchase_tax: form.purchase_tax,
        nickname: form.nickname,
        barcodes: Vec::new(),
        sub_products: Vec::new(),
        category: form.category,
        print_lists: form.print_lists,
        tags: form.tags,
        image: None,
        status_prices,
        stocked: form.stocked,
        target_quantity: form.target_quantity,
    };

    let product = save_product(&mut state, product, children).await?;
    Ok(Json(ProductDto::from(&product)))
}

fn create_product_docs(op: TransformOperation) -> TransformOperation {
    op.description("Create a new product.")
        .tag("products")
        .response::<200, Json<ProductDto>>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn update_product(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveProductDto>,
) -> ServiceResult<Json<ProductDto>> {
    state.session_require_purchaser_or_admin()?;

    let form = form.0;
    let product = state.db.get_product_by_id(id).await?;

    let status_prices = resolve_status_prices(&mut state, &form.status_prices).await?;
    let children = ProductChildren {
        barcode: None,
        barcodes: form.barcodes,
        sub_products: form.sub_products,
    };

    if let Some(mut product) = product {
        product.name = form.name;
        product.price = form.price.into();
        product.bonus = form.bonus.into();
        product.nickname = form.nickname;
        product.purchase_tax = form.purchase_tax;
        product.category = form.category;
        product.print_lists = form.print_lists;
        product.tags = form.tags;
        product.status_prices = status_prices;
        product.stocked = form.stocked;
        product.target_quantity = form.target_quantity;

        let product = save_product(&mut state, product, children).await?;
        return Ok(Json(ProductDto::from(&product)));
    }

    Err(ServiceError::NotFound)
}

fn update_product_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update an existing product.")
        .tag("products")
        .response::<200, Json<ProductDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested product does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn delete_product(mut state: RequestState, Path(id): Path<u64>) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;

    state.db.delete_product(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_product_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete an existing product.")
        .tag("products")
        .response_with::<204, (), _>(|res| res.description("The product was successfully deleted!"))
        .response_with::<404, (), _>(|res| res.description("The requested product does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

pub async fn get_product_by_barcode(
    mut state: RequestState,
    Path(barcode): Path<String>,
) -> ServiceResult<Json<ProductDto>> {
    let product = state.db.get_product_by_barcode(barcode.trim()).await?;

    if let Some(product) = product {
        return Ok(Json(ProductDto::from(&product)));
    }

    Err(ServiceError::NotFound)
}

fn get_product_by_barcode_docs(op: TransformOperation) -> TransformOperation {
    op.description("Find a product by its barcode (exact match after trimming whitespace).")
        .tag("products")
        .response::<200, Json<ProductDto>>()
        .response_with::<404, (), _>(|res| res.description("No product has this barcode!"))
}

/// A product that already uses a barcode, for the duplicate warning in the product form.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct BarcodeConflictDto {
    pub product_id: u64,
    pub product_name: String,
}

pub async fn get_barcode_conflicts(
    mut state: RequestState,
    Path(barcode): Path<String>,
) -> ServiceResult<Json<Vec<BarcodeConflictDto>>> {
    let products = state.db.get_products_with_barcode(barcode.trim()).await?;
    Ok(Json(
        products
            .into_iter()
            .map(|(product_id, product_name)| BarcodeConflictDto {
                product_id,
                product_name,
            })
            .collect(),
    ))
}

fn get_barcode_conflicts_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all products that use the given barcode. Barcodes are not unique; this powers the duplicate warning in the product form.")
        .tag("products")
        .response::<200, Json<Vec<BarcodeConflictDto>>>()
}

async fn add_product_barcode(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveProductBarcodeDto>,
) -> ServiceResult<(StatusCode, Json<ProductBarcodeDto>)> {
    state.session_require_purchaser_or_admin()?;

    let form = form.0;
    if form.sub_product_key.is_some() {
        return Err(ServiceError::BadRequest(
            "sub_product_key is only supported when saving the whole product".to_owned(),
        ));
    }
    let barcode = state
        .db
        .add_product_barcode(
            id,
            &form.code,
            form.container_size,
            &form.label,
            form.sub_product_id,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(ProductBarcodeDto::from(&barcode))))
}

fn add_product_barcode_docs(op: TransformOperation) -> TransformOperation {
    op.description("Add a single barcode to an existing product, without rewriting the product.")
        .tag("products")
        .response::<201, Json<ProductBarcodeDto>>()
        .response_with::<400, (), _>(|res| {
            res.description("The sub product does not belong to this product!")
        })
        .response_with::<404, (), _>(|res| res.description("The requested product does not exist!"))
        .response_with::<409, (), _>(|res| res.description("The product already has this barcode!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn delete_product_barcode(
    mut state: RequestState,
    Path((id, barcode_id)): Path<(u64, u64)>,
) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;

    state.db.delete_product_barcode(id, barcode_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_product_barcode_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete a single barcode of a product.")
        .tag("products")
        .response_with::<204, (), _>(|res| res.description("The barcode was successfully deleted!"))
        .response_with::<404, (), _>(|res| {
            res.description("The barcode does not belong to this product!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn add_sub_product(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveSubProductDto>,
) -> ServiceResult<(StatusCode, Json<SubProductDto>)> {
    state.session_require_purchaser_or_admin()?;

    let product = state
        .db
        .get_product_by_id(id)
        .await?
        .ok_or(ServiceError::NotFound)?;
    let mut sub_product = form.0.to_model(product.purchase_tax);
    sub_product.id = 0;

    let sub_product = state.db.add_sub_product(id, &sub_product).await?;
    Ok((StatusCode::CREATED, Json(SubProductDto::from(&sub_product))))
}

fn add_sub_product_docs(op: TransformOperation) -> TransformOperation {
    op.description("Add a sub product to an existing product, without rewriting the product. `id` and `key` are ignored; `purchase_tax` defaults to the product's.")
        .tag("products")
        .response::<201, Json<SubProductDto>>()
        .response_with::<400, (), _>(|res| res.description("The name is empty!"))
        .response_with::<404, (), _>(|res| res.description("The requested product does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn update_sub_product(
    mut state: RequestState,
    Path((id, sub_product_id)): Path<(u64, u64)>,
    form: Json<SaveSubProductDto>,
) -> ServiceResult<Json<SubProductDto>> {
    state.session_require_purchaser_or_admin()?;

    let product = state
        .db
        .get_product_by_id(id)
        .await?
        .ok_or(ServiceError::NotFound)?;
    let mut sub_product = form.0.to_model(product.purchase_tax);
    sub_product.id = sub_product_id;

    let sub_product = state.db.update_sub_product(id, &sub_product).await?;
    Ok(Json(SubProductDto::from(&sub_product)))
}

fn update_sub_product_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Rename, re-tax or (de)activate a sub product. `id` and `key` of the body are ignored.",
    )
    .tag("products")
    .response::<200, Json<SubProductDto>>()
    .response_with::<400, (), _>(|res| res.description("The name is empty!"))
    .response_with::<404, (), _>(|res| {
        res.description("The sub product does not belong to this product!")
    })
    .response_with::<401, (), _>(|res| res.description("Missing login!"))
    .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
    .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct UpdateInventoryProductDto {
    pub product_id: u64,
    pub stocked: bool,
    pub target_quantity: i32,
}

async fn update_products_inventory(
    mut state: RequestState,
    form: Json<Vec<UpdateInventoryProductDto>>,
) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;

    let updates: Vec<(u64, bool, i32)> = form
        .0
        .iter()
        .map(|u| (u.product_id, u.stocked, u.target_quantity))
        .collect();

    state.db.update_products_inventory(&updates).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn update_products_inventory_docs(op: TransformOperation) -> TransformOperation {
    op.description("Bulk-update the inventory fields (stocked, target quantity) of products. Nothing is applied if any product id is unknown.")
        .tag("products")
        .response_with::<204, (), _>(|res| res.description("The products were updated!"))
        .response_with::<404, (), _>(|res| res.description("A referenced product does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn resolve_status_prices(
    state: &mut RequestState,
    status_prices_dto: &[SaveProductStatusPriceDto],
) -> ServiceResult<Vec<models::ProductStatusPrice>> {
    let mut status_prices: Vec<models::ProductStatusPrice> = Vec::new();
    for status_price_dto in status_prices_dto {
        if let Some(status) = state
            .db
            .get_account_status_by_id(status_price_dto.status_id)
            .await?
        {
            status_prices.push(models::ProductStatusPrice {
                status,
                price: (&status_price_dto.price).into(),
                bonus: (&status_price_dto.bonus).into(),
            });
        }
    }

    Ok(status_prices)
}

pub async fn get_product_image(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<ImageResult> {
    let image = state.db.get_product_image(id).await?;

    if let Some(image) = image {
        if let Ok(content_type) = HeaderValue::from_str(&image.mimetype) {
            return Ok(ImageResult {
                content_type,
                body: image.data,
            });
        }
    }

    Err(ServiceError::NotFound)
}

fn get_product_image_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get the image of the given product.")
        .tag("product_image")
        .response::<200, Bytes>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested product image does not exist!")
        })
}

async fn upload_product_image(
    mut state: RequestState,
    Path(id): Path<u64>,
    mut multipart: Multipart,
) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;
    while let Ok(Some(field)) = multipart.next_field().await {
        let content_type = field.content_type().unwrap_or("").to_lowercase();
        if SUPPORTED_IMAGE_TYPES.iter().any(|t| *t == content_type) {
            if let Ok(data) = field.bytes().await {
                let image = models::Image {
                    data: data.to_vec(),
                    mimetype: content_type,
                };
                state.db.store_product_image(id, image).await?;
                return Ok(StatusCode::NO_CONTENT);
            }
        }
    }

    Err(ServiceError::NotFound)
}

fn upload_product_image_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update the image of the given product.")
        .tag("product_image")
        .response_with::<204, (), _>(|res| {
            res.description("The product image was successfully updated!")
        })
        .response_with::<404, (), _>(|res| res.description("The requested product does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn delete_product_image(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;

    state.db.delete_product_image(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_product_image_docs(op: TransformOperation) -> TransformOperation {
    op.description("Remove the image from the given product.")
        .tag("product_image")
        .response_with::<204, (), _>(|res| {
            res.description("The product image was successfully deleted!")
        })
        .response_with::<404, (), _>(|res| res.description("The requested product does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

pub struct ImageResult {
    pub content_type: HeaderValue,
    pub body: Vec<u8>,
}

impl OperationOutput for ImageResult {
    type Inner = Bytes;
}
impl IntoResponse for ImageResult {
    fn into_response(self) -> axum::response::Response {
        let mut header = HeaderMap::new();
        header.insert(header::CONTENT_TYPE, self.content_type);
        (StatusCode::OK, header, self.body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{Duration, Utc};
    use sqlx::PgPool;

    use super::*;
    use crate::database::{AppState, DatabaseConnection};
    use crate::models::{Account, AuthMethodType, CoinAmount, CoinType, Role, Session};

    async fn request_state(app_state: &AppState, account: Option<Account>) -> RequestState {
        let session = account.map(|account| Session {
            account,
            token: "test".to_string(),
            auth_method: AuthMethodType::PasswordBased,
            valid_until: Utc::now() + Duration::minutes(30),
            is_single_use: false,
        });
        RequestState {
            db: DatabaseConnection {
                connection: app_state.pool.acquire().await.unwrap(),
            },
            session,
            challenge_storage: app_state.ascii_mifare_challenge.clone(),
        }
    }

    fn product(name: &str, barcode: Option<&str>) -> models::Product {
        models::Product {
            id: 0,
            name: name.to_string(),
            price: CoinAmount([(CoinType::Cent, 100)].into_iter().collect()),
            bonus: CoinAmount(HashMap::new()),
            purchase_tax: 19,
            nickname: None,
            image: None,
            barcodes: barcode
                .map(|code| models::ProductBarcode {
                    id: 0,
                    code: code.to_string(),
                    container_size: 1,
                    label: String::new(),
                    sub_product_id: None,
                })
                .into_iter()
                .collect(),
            sub_products: Vec::new(),
            category: "cat".to_string(),
            print_lists: vec![],
            tags: vec![],
            status_prices: vec![],
            stocked: false,
            target_quantity: 0,
        }
    }

    #[sqlx::test]
    async fn by_barcode_trims_and_404s(pool: PgPool) {
        let app_state = AppState::from_pool(pool).await;
        let mut db = DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        };
        let first = db
            .store_product(product("First", Some("4711")))
            .await
            .unwrap();
        let _second = db
            .store_product(product("Second", Some("4711")))
            .await
            .unwrap();

        let state = request_state(&app_state, None).await;
        let found = get_product_by_barcode(state, Path("4711".to_string()))
            .await
            .unwrap();
        assert_eq!(found.0.id, first.id);

        let state = request_state(&app_state, None).await;
        let found = get_product_by_barcode(state, Path("  4711\n".to_string()))
            .await
            .unwrap();
        assert_eq!(found.0.id, first.id);

        let state = request_state(&app_state, None).await;
        assert_eq!(
            get_product_by_barcode(state, Path("471".to_string()))
                .await
                .err(),
            Some(ServiceError::NotFound)
        );
    }

    #[sqlx::test]
    async fn bulk_inventory_update(pool: PgPool) {
        let app_state = AppState::from_pool(pool).await;
        let mut db = DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        };
        let admin = db
            .store_account(Account {
                id: 0,
                balance: CoinAmount(HashMap::new()),
                name: "admin".to_string(),
                email: "admin@example.org".to_string(),
                role: Role::Admin,
                auth_methods: vec![],
                enable_monthly_mail_report: false,
                enable_automatic_stamp_usage: false,
                status: None,
            })
            .await
            .unwrap();
        let p1 = db.store_product(product("P1", None)).await.unwrap();
        let p2 = db.store_product(product("P2", None)).await.unwrap();

        let updates = |unknown: bool| {
            let mut v = vec![
                UpdateInventoryProductDto {
                    product_id: p1.id,
                    stocked: true,
                    target_quantity: 7,
                },
                UpdateInventoryProductDto {
                    product_id: p2.id,
                    stocked: true,
                    target_quantity: 3,
                },
            ];
            if unknown {
                v.push(UpdateInventoryProductDto {
                    product_id: 424242,
                    stocked: true,
                    target_quantity: 1,
                });
            }
            v
        };

        let state = request_state(&app_state, None).await;
        assert_eq!(
            update_products_inventory(state, Json(updates(false)))
                .await
                .err(),
            Some(ServiceError::Unauthorized("Missing login!"))
        );

        let state = request_state(&app_state, Some(admin.clone())).await;
        assert_eq!(
            update_products_inventory(state, Json(updates(true)))
                .await
                .err(),
            Some(ServiceError::NotFound)
        );
        assert!(!db.get_product_by_id(p1.id).await.unwrap().unwrap().stocked);

        let state = request_state(&app_state, Some(admin.clone())).await;
        assert_eq!(
            update_products_inventory(state, Json(updates(false))).await,
            Ok(StatusCode::NO_CONTENT)
        );
        let p1 = db.get_product_by_id(p1.id).await.unwrap().unwrap();
        let p2 = db.get_product_by_id(p2.id).await.unwrap().unwrap();
        assert_eq!((p1.stocked, p1.target_quantity), (true, 7));
        assert_eq!((p2.stocked, p2.target_quantity), (true, 3));
    }

    fn admin_account() -> Account {
        Account {
            id: 0,
            balance: CoinAmount(HashMap::new()),
            name: "admin".to_string(),
            email: "admin2@example.org".to_string(),
            role: Role::Admin,
            auth_methods: vec![],
            enable_monthly_mail_report: false,
            enable_automatic_stamp_usage: false,
            status: None,
        }
    }

    fn save_product(barcode: Option<&str>, barcodes: Option<Vec<(&str, i32)>>) -> SaveProductDto {
        SaveProductDto {
            name: "Mate".to_string(),
            price: CoinAmountDto::default(),
            bonus: CoinAmountDto::default(),
            purchase_tax: 19,
            nickname: None,
            barcode: barcode.map(str::to_string),
            barcodes: barcodes.map(|list| {
                list.into_iter()
                    .map(|(code, size)| SaveProductBarcodeDto {
                        code: code.to_string(),
                        container_size: size,
                        label: String::new(),
                        sub_product_id: None,
                        sub_product_key: None,
                    })
                    .collect()
            }),
            sub_products: None,
            category: "Drinks".to_string(),
            print_lists: vec![],
            tags: vec![],
            status_prices: vec![],
            stocked: false,
            target_quantity: 0,
        }
    }

    fn codes(product: &ProductDto) -> Vec<(&str, i32)> {
        product
            .barcodes
            .iter()
            .map(|b| (b.code.as_str(), b.container_size))
            .collect()
    }

    /// `barcodes` replaces the whole list; a request without the field never drops variants it
    /// does not know about (the legacy `barcode` field is only a create-time shorthand).
    #[sqlx::test]
    async fn save_dto_barcode_compatibility(pool: PgPool) {
        let app_state = AppState::from_pool(pool).await;
        let admin = DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        }
        .store_account(admin_account())
        .await
        .unwrap();

        // an old client creating a product still gets its single code
        let state = request_state(&app_state, Some(admin.clone())).await;
        let legacy = create_product(state, Json(save_product(Some("4001"), None)))
            .await
            .unwrap()
            .0;
        assert_eq!(codes(&legacy), vec![("4001", 1)]);
        assert_eq!(legacy.barcode.as_deref(), Some("4001"));

        // a new client sends the full list
        let state = request_state(&app_state, Some(admin.clone())).await;
        let created = create_product(
            state,
            Json(save_product(None, Some(vec![("4711", 1), ("4712", 20)]))),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(codes(&created), vec![("4711", 1), ("4712", 20)]);
        // the legacy field mirrors the primary code
        assert_eq!(created.barcode.as_deref(), Some("4711"));

        // an old client echoing only `barcode` must not drop the crate code
        let state = request_state(&app_state, Some(admin.clone())).await;
        let updated = update_product(
            state,
            Path(created.id),
            Json(save_product(Some("4711"), None)),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(codes(&updated), vec![("4711", 1), ("4712", 20)]);

        // sending the list replaces it
        let state = request_state(&app_state, Some(admin.clone())).await;
        let updated = update_product(
            state,
            Path(created.id),
            Json(save_product(None, Some(vec![("4712", 24)]))),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(codes(&updated), vec![("4712", 24)]);

        // and an empty list clears them
        let state = request_state(&app_state, Some(admin.clone())).await;
        let updated = update_product(
            state,
            Path(created.id),
            Json(save_product(None, Some(vec![]))),
        )
        .await
        .unwrap()
        .0;
        assert!(updated.barcodes.is_empty());
        assert_eq!(updated.barcode, None);
    }

    fn save_sub_product(id: Option<u64>, key: Option<&str>, name: &str) -> SaveSubProductDto {
        SaveSubProductDto {
            id,
            key: key.map(str::to_string),
            name: name.to_string(),
            purchase_tax: None,
            active: true,
            show_on_print_list: false,
            target_quantity: 0,
        }
    }

    fn save_barcode(
        code: &str,
        sub_product_id: Option<u64>,
        key: Option<&str>,
    ) -> SaveProductBarcodeDto {
        SaveProductBarcodeDto {
            code: code.to_string(),
            container_size: 1,
            label: String::new(),
            sub_product_id,
            sub_product_key: key.map(str::to_string),
        }
    }

    /// One save creates the product, its sub products and the codes pointing at them.
    #[sqlx::test]
    async fn save_dto_sub_products(pool: PgPool) {
        let app_state = AppState::from_pool(pool).await;
        let admin = DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        }
        .store_account(admin_account())
        .await
        .unwrap();

        let mut form = save_product(None, None);
        form.purchase_tax = 7;
        form.sub_products = Some(vec![
            save_sub_product(None, Some("new-1"), "Goldbären"),
            SaveSubProductDto {
                purchase_tax: Some(19),
                ..save_sub_product(None, Some("new-2"), "Tropifrutti")
            },
        ]);
        form.barcodes = Some(vec![
            save_barcode("4001", None, Some("new-1")),
            save_barcode("4002", None, Some("new-2")),
            save_barcode("4000", None, None),
        ]);
        let state = request_state(&app_state, Some(admin.clone())).await;
        let created = create_product(state, Json(form)).await.unwrap().0;

        let gold = &created.sub_products[0];
        let tropi = &created.sub_products[1];
        assert_eq!((gold.name.as_str(), gold.purchase_tax), ("Goldbären", 7));
        assert_eq!(
            (tropi.name.as_str(), tropi.purchase_tax),
            ("Tropifrutti", 19)
        );
        assert_eq!(
            created
                .barcodes
                .iter()
                .map(|b| (b.code.as_str(), b.sub_product_id))
                .collect::<Vec<_>>(),
            vec![
                ("4001", Some(gold.id)),
                ("4002", Some(tropi.id)),
                ("4000", None)
            ]
        );

        // an update can mix existing ids with a new key; leaving out `sub_products`
        // keeps them untouched
        let mut form = save_product(None, None);
        form.sub_products = Some(vec![
            save_sub_product(Some(gold.id), None, "Goldbären 175g"),
            save_sub_product(Some(tropi.id), None, "Tropifrutti"),
            save_sub_product(None, Some("x"), "Color-Rado"),
        ]);
        form.barcodes = Some(vec![
            save_barcode("4001", Some(gold.id), None),
            save_barcode("4003", None, Some("x")),
        ]);
        let state = request_state(&app_state, Some(admin.clone())).await;
        let updated = update_product(state, Path(created.id), Json(form))
            .await
            .unwrap()
            .0;
        assert_eq!(updated.sub_products.len(), 3);
        assert_eq!(updated.sub_products[0].name, "Goldbären 175g");
        let rado = updated.sub_products[2].id;
        assert_eq!(updated.barcodes[1].sub_product_id, Some(rado));

        let state = request_state(&app_state, Some(admin.clone())).await;
        let unchanged = update_product(state, Path(created.id), Json(save_product(None, None)))
            .await
            .unwrap()
            .0;
        assert_eq!(unchanged.sub_products, updated.sub_products);
        assert_eq!(unchanged.barcodes[1].sub_product_id, Some(rado));

        // unknown keys and ids of other products are rejected
        let mut form = save_product(None, None);
        form.barcodes = Some(vec![save_barcode("4001", None, Some("nope"))]);
        let state = request_state(&app_state, Some(admin.clone())).await;
        assert!(matches!(
            update_product(state, Path(created.id), Json(form)).await,
            Err(ServiceError::BadRequest(_))
        ));

        let state = request_state(&app_state, Some(admin.clone())).await;
        let other = create_product(state, Json(save_product(None, None)))
            .await
            .unwrap()
            .0;
        let mut form = save_product(None, None);
        form.barcodes = Some(vec![save_barcode("5000", Some(rado), None)]);
        let state = request_state(&app_state, Some(admin.clone())).await;
        assert!(matches!(
            update_product(state, Path(other.id), Json(form)).await,
            Err(ServiceError::BadRequest(_))
        ));

        // the single endpoints
        let state = request_state(&app_state, Some(admin.clone())).await;
        let (status, added) = add_sub_product(
            state,
            Path(created.id),
            Json(save_sub_product(Some(999), None, "Phantasia")),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_ne!(added.id, 999);
        // inherits the parent's current tax (the updates above sent 19)
        assert_eq!(added.purchase_tax, 19);

        let state = request_state(&app_state, Some(admin.clone())).await;
        let deactivated = update_sub_product(
            state,
            Path((created.id, added.id)),
            Json(SaveSubProductDto {
                active: false,
                ..save_sub_product(None, None, "Phantasia")
            }),
        )
        .await
        .unwrap()
        .0;
        assert!(!deactivated.active);

        let state = request_state(&app_state, Some(admin.clone())).await;
        assert!(matches!(
            update_sub_product(
                state,
                Path((other.id, added.id)),
                Json(save_sub_product(None, None, "Phantasia")),
            )
            .await,
            Err(ServiceError::NotFound)
        ));

        // purchase and shopping list items only accept the product's own sub products
        let parent = state_product(&app_state, created.id).await;
        assert_eq!(
            resolve_sub_product(Some(&parent), Some(rado)),
            Ok(Some(rado))
        );
        assert_eq!(resolve_sub_product(Some(&parent), None), Ok(None));
        assert!(resolve_sub_product(None, Some(rado)).is_err());
        let other = state_product(&app_state, other.id).await;
        assert!(resolve_sub_product(Some(&other), Some(rado)).is_err());
    }

    async fn state_product(app_state: &AppState, id: u64) -> models::Product {
        DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        }
        .get_product_by_id(id)
        .await
        .unwrap()
        .unwrap()
    }

    #[sqlx::test]
    async fn barcode_endpoints(pool: PgPool) {
        let app_state = AppState::from_pool(pool).await;
        let mut db = DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        };
        let admin = db.store_account(admin_account()).await.unwrap();
        let mate = db
            .store_product(product("Mate", Some("4711")))
            .await
            .unwrap();
        let cola = db
            .store_product(product("Cola", Some("4711")))
            .await
            .unwrap();

        // the duplicate warning sees both products
        let state = request_state(&app_state, None).await;
        let conflicts = get_barcode_conflicts(state, Path(" 4711 ".to_string()))
            .await
            .unwrap()
            .0;
        assert_eq!(
            conflicts
                .iter()
                .map(|c| (c.product_id, c.product_name.as_str()))
                .collect::<Vec<_>>(),
            vec![(mate.id, "Mate"), (cola.id, "Cola")]
        );

        let body = || {
            Json(SaveProductBarcodeDto {
                code: "4712".to_string(),
                container_size: 20,
                label: "Kasten".to_string(),
                sub_product_id: None,
                sub_product_key: None,
            })
        };

        let state = request_state(&app_state, None).await;
        assert_eq!(
            add_product_barcode(state, Path(mate.id), body())
                .await
                .err(),
            Some(ServiceError::Unauthorized("Missing login!"))
        );

        let state = request_state(&app_state, Some(admin.clone())).await;
        let (status, added) = add_product_barcode(state, Path(mate.id), body())
            .await
            .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(
            (added.0.code.as_str(), added.0.container_size),
            ("4712", 20)
        );

        // scanning the new code now finds the product
        let state = request_state(&app_state, None).await;
        let found = get_product_by_barcode(state, Path("4712".to_string()))
            .await
            .unwrap()
            .0;
        assert_eq!(found.id, mate.id);
        assert_eq!(codes(&found), vec![("4711", 1), ("4712", 20)]);

        let state = request_state(&app_state, Some(admin.clone())).await;
        assert_eq!(
            add_product_barcode(state, Path(mate.id), body())
                .await
                .err(),
            Some(ServiceError::Conflict(
                "the product already has this barcode"
            ))
        );
        let state = request_state(&app_state, Some(admin.clone())).await;
        assert_eq!(
            add_product_barcode(state, Path(424242), body()).await.err(),
            Some(ServiceError::NotFound)
        );

        let barcode_id = added.0.id;
        let state = request_state(&app_state, None).await;
        assert_eq!(
            delete_product_barcode(state, Path((mate.id, barcode_id)))
                .await
                .err(),
            Some(ServiceError::Unauthorized("Missing login!"))
        );
        let state = request_state(&app_state, Some(admin.clone())).await;
        assert_eq!(
            delete_product_barcode(state, Path((cola.id, barcode_id)))
                .await
                .err(),
            Some(ServiceError::NotFound)
        );
        let state = request_state(&app_state, Some(admin.clone())).await;
        assert_eq!(
            delete_product_barcode(state, Path((mate.id, barcode_id))).await,
            Ok(StatusCode::NO_CONTENT)
        );
        let state = request_state(&app_state, None).await;
        assert_eq!(
            get_product_by_barcode(state, Path("4712".to_string()))
                .await
                .err(),
            Some(ServiceError::NotFound)
        );
    }
}
