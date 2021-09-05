use crate::{
    identity_service::{Identity, IdentityMut, IdentityRequire},
    model::{authentication_password, Permission},
    utils::{DatabaseConnection, RedisConnection, ServiceError, ServiceResult},
};

use super::accounts::AccountOutput;

#[derive(Debug, Deserialize, InputObject)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct LoginOutput {
    pub token: String,
    pub authorization: String,
}

pub fn get_me(identity: &Identity) -> ServiceResult<AccountOutput> {
    let entity = identity.require_account(Permission::Default)?;
    Ok(entity.into())
}

pub fn login(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    identity: &Identity,
    input: LoginInput,
) -> ServiceResult<LoginOutput> {
    let login_result =
        authentication_password::get(database_conn, &input.username, &input.password);
    match login_result {
        Ok(account) => {
            identity.store(database_conn, redis_conn, account.id)?;

            let token = identity.require_auth_token()?;
            Ok(LoginOutput {
                authorization: format!("Bearer {}", &token),
                token,
            })
        }
        Err(_) => Err(ServiceError::Unauthorized),
    }
}

pub fn login_mut(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    identity: &IdentityMut,
    input: LoginInput,
) -> ServiceResult<LoginOutput> {
    let login_result =
        authentication_password::get(database_conn, &input.username, &input.password);
    match login_result {
        Ok(account) => {
            identity.store(database_conn, redis_conn, account.id)?;

            let token = identity.require_auth_token()?;
            Ok(LoginOutput {
                authorization: format!("Bearer {}", &token),
                token,
            })
        }
        Err(_) => Err(ServiceError::Unauthorized),
    }
}

pub fn logout(conn: &mut RedisConnection, identity: &Identity) -> ServiceResult<()> {
    identity.forget(conn)?;
    Ok(())
}

pub fn logout_mut(conn: &mut RedisConnection, identity: &IdentityMut) -> ServiceResult<()> {
    identity.forget(conn)?;
    Ok(())
}
