use aide::axum::routing::get_with;
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

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ProductDto {
    pub id: u64,
    pub name: String,
    pub price: CoinAmountDto,
    pub bonus: CoinAmountDto,
    pub purchase_tax: i32,
    pub nickname: Option<String>,
    pub barcode: Option<String>,
    pub category: String,
    pub print_lists: Vec<String>,
    pub tags: Vec<String>,
    pub status_prices: Vec<ProductStatusPriceDto>,
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
            barcode: value.barcode.to_owned(),
            category: value.category.to_owned(),
            print_lists: value.print_lists.to_owned(),
            tags: value.tags.to_owned(),
            status_prices: value
                .status_prices
                .iter()
                .map(ProductStatusPriceDto::from)
                .collect(),
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
pub struct SaveProductDto {
    pub name: String,
    pub price: CoinAmountDto,
    pub bonus: CoinAmountDto,
    pub purchase_tax: i32,
    pub nickname: Option<String>,
    pub barcode: Option<String>,
    pub category: String,
    pub print_lists: Vec<String>,
    pub tags: Vec<String>,
    pub status_prices: Vec<SaveProductStatusPriceDto>,
}

async fn create_product(
    mut state: RequestState,
    form: Json<SaveProductDto>,
) -> ServiceResult<Json<ProductDto>> {
    state.session_require_purchaser_or_admin()?;

    let form = form.0;

    let status_prices = resolve_status_prices(&mut state, &form.status_prices).await?;

    let product = models::Product {
        id: 0,
        name: form.name,
        price: form.price.into(),
        bonus: form.bonus.into(),
        purchase_tax: form.purchase_tax,
        nickname: form.nickname,
        barcode: form.barcode,
        category: form.category,
        print_lists: form.print_lists,
        tags: form.tags,
        image: None,
        status_prices,
    };

    let product = state.db.store_product(product).await?;
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

    if let Some(mut product) = product {
        product.name = form.name;
        product.price = form.price.into();
        product.bonus = form.bonus.into();
        product.nickname = form.nickname;
        product.purchase_tax = form.purchase_tax;
        product.barcode = form.barcode;
        product.category = form.category;
        product.print_lists = form.print_lists;
        product.tags = form.tags;
        product.status_prices = status_prices;

        let product = state.db.store_product(product).await?;
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
