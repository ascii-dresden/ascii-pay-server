use std::convert::TryFrom;

use async_graphql::{Context, Object};
use uuid::Uuid;

use crate::{
    api::rest::auth::LoginForm,
    core::{self, authentication_password, Permission, ServiceError, ServiceResult},
    identity_service::{Identity, IdentityRequire},
};

use super::{
    get_conn_from_ctx,
    model::{
        Category, CategoryCreateInput, CategoryUpdateInput, LoginResult, Product,
        ProductCreateInput, ProductUpdateInput,
    },
};

pub struct Mutation;

#[Object]
impl Mutation {
    async fn login(&self, ctx: &Context<'_>, input: LoginForm) -> ServiceResult<LoginResult> {
        let identity = ctx.data::<Identity>()?;

        let conn = &get_conn_from_ctx(ctx)?;

        let login_result = authentication_password::get(conn, &input.username, &input.password);
        match login_result {
            Ok(account) => {
                identity.store(&conn, &account.id)?;

                let token = identity.require_auth_token()?;
                Ok(LoginResult {
                    authorization: format!("Bearer {}", &token),
                    token,
                })
            }
            Err(_) => Err(ServiceError::Unauthorized),
        }
    }

    async fn logout(&self, ctx: &Context<'_>) -> ServiceResult<String> {
        let identity = ctx.data::<Identity>()?;

        let conn = &get_conn_from_ctx(ctx)?;

        identity.forget(&conn)?;

        Ok("ok".to_owned())
    }

    async fn create_category(
        &self,
        ctx: &Context<'_>,
        category: CategoryCreateInput,
    ) -> ServiceResult<Category> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;
        let mut entity = core::Category::create(conn, &category.name)?;
        entity.update_prices(
            &conn,
            &category
                .prices
                .iter()
                .map(|p| p.into())
                .collect::<Vec<core::Price>>(),
        )?;

        Ok(Category::from(&entity))
    }

    async fn update_category(
        &self,
        ctx: &Context<'_>,
        category: CategoryUpdateInput,
    ) -> ServiceResult<Category> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;
        let mut entity = core::Category::get(conn, &Uuid::try_from(category.id)?)?;

        entity.name = category.name.clone();
        entity.update(&conn)?;
        entity.update_prices(
            &conn,
            &category
                .prices
                .iter()
                .map(|p| p.into())
                .collect::<Vec<core::Price>>(),
        )?;

        Ok(Category::from(&entity))
    }

    async fn create_product(
        &self,
        ctx: &Context<'_>,
        product: ProductCreateInput,
    ) -> ServiceResult<Product> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;

        let category = if let Some(x) = product.category {
            Some(core::Category::get(&conn, &Uuid::try_from(x)?)?)
        } else {
            None
        };
        let mut entity = core::Product::create(conn, &product.name, category)?;
        entity.update_prices(
            &conn,
            &product
                .prices
                .iter()
                .map(|p| p.into())
                .collect::<Vec<core::Price>>(),
        )?;

        Ok(Product::from(&entity))
    }

    async fn update_product(
        &self,
        ctx: &Context<'_>,
        product: ProductUpdateInput,
    ) -> ServiceResult<Product> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;

        let category = if let Some(x) = product.category {
            Some(core::Category::get(&conn, &Uuid::try_from(x)?)?)
        } else {
            None
        };
        let mut entity = core::Product::get(conn, &Uuid::try_from(product.id)?)?;

        entity.name = product.name.clone();
        entity.category = category;
        entity.update(&conn)?;
        entity.update_prices(
            &conn,
            &product
                .prices
                .iter()
                .map(|p| p.into())
                .collect::<Vec<core::Price>>(),
        )?;

        Ok(Product::from(&entity))
    }
}
