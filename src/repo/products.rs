use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{Permission, Product};
use crate::utils::ServiceResult;

pub fn get_products(_identity: &Identity) -> ServiceResult<Vec<Product>> {
    Product::all()
}

pub fn get_product(_identity: &Identity, id: &str) -> ServiceResult<Product> {
    Product::get(id)
}

pub fn get_product_image(_identity: &Identity, id: &str) -> ServiceResult<String> {
    Product::get_image(id)
}

pub fn update_products(identity: &Identity) -> ServiceResult<()> {
    identity.require_account(Permission::Admin)?;
    Product::load_dataset()?;
    Ok(())
}
