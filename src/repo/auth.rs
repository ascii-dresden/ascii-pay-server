use crate::{
    identity_service::{Identity, IdentityMut, IdentityRequire},
    model::{
        authentication_password,
        session::{get_onetime_session, Session},
        Permission,
    },
    utils::{DatabasePool, RedisPool, ServiceError, ServiceResult},
};

use super::accounts::AccountOutput;

#[derive(Debug, Deserialize, InputObject)]
pub struct LoginInput {
    pub username: Option<String>,
    pub password: Option<String>,
    pub account_access_token: Option<Session>,
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

pub async fn login(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    identity: &Identity,
    input: LoginInput,
) -> ServiceResult<LoginOutput> {
    if let Some(account_access_token) = input.account_access_token {
        let login_result =
            get_onetime_session(database_pool, redis_pool, &account_access_token).await;

        return match login_result {
            Ok(account) => {
                identity
                    .store(database_pool, redis_pool, account.id)
                    .await?;

                let token = identity.require_auth_token()?;
                Ok(LoginOutput {
                    authorization: format!("Bearer {}", &token),
                    token,
                })
            }
            Err(_) => Err(ServiceError::Unauthorized("invalid onetime session")),
        };
    }

    let username = input.username.unwrap_or_default();
    let password = input.password.unwrap_or_default();

    let login_result = authentication_password::get(database_pool, &username, &password).await;
    match login_result {
        Ok(account) => {
            identity
                .store(database_pool, redis_pool, account.id)
                .await?;

            let token = identity.require_auth_token()?;
            Ok(LoginOutput {
                authorization: format!("Bearer {}", &token),
                token,
            })
        }
        Err(_) => Err(ServiceError::Unauthorized("invalid username/password")),
    }
}

pub async fn login_mut(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    identity: &IdentityMut,
    input: LoginInput,
) -> ServiceResult<LoginOutput> {
    if let Some(account_access_token) = input.account_access_token {
        let login_result =
            get_onetime_session(database_pool, redis_pool, &account_access_token).await;

        return match login_result {
            Ok(account) => {
                identity
                    .store(database_pool, redis_pool, account.id)
                    .await?;

                let token = identity.require_auth_token()?;
                Ok(LoginOutput {
                    authorization: format!("Bearer {}", &token),
                    token,
                })
            }
            Err(_) => Err(ServiceError::Unauthorized("invalid onetime session")),
        };
    }

    let username = input.username.unwrap_or_default();
    let password = input.password.unwrap_or_default();

    let login_result = authentication_password::get(database_pool, &username, &password).await;
    match login_result {
        Ok(account) => {
            identity
                .store(database_pool, redis_pool, account.id)
                .await?;

            let token = identity.require_auth_token()?;
            Ok(LoginOutput {
                authorization: format!("Bearer {}", &token),
                token,
            })
        }
        Err(_) => Err(ServiceError::Unauthorized("invalid username/password")),
    }
}

pub async fn logout(redis_pool: &RedisPool, identity: &Identity) -> ServiceResult<()> {
    identity.forget(redis_pool).await?;
    Ok(())
}

pub async fn logout_mut(redis_pool: &RedisPool, identity: &IdentityMut) -> ServiceResult<()> {
    identity.forget(redis_pool).await?;
    Ok(())
}
