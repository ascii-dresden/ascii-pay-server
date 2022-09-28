use uuid::Uuid;

use crate::{
    identity_service::{Identity, IdentityMut, IdentityRequire},
    model::{
        authentication_password,
        session::{get_onetime_session, Session},
        Account, Permission,
    },
    utils::{DatabasePool, RedisPool, ServiceError, ServiceResult},
};

use super::{accounts::AccountOutput, authenticate_nfc_mifare_desfire_login};

#[derive(Debug, Deserialize, InputObject)]
pub struct LoginInput {
    pub username: Option<String>,
    pub password: Option<String>,
    pub account_access_token: Option<Session>,
    pub nfc_card_id: Option<String>,
    pub nfc_card_secret: Option<String>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct LoginOutput {
    pub token: String,
    pub authorization: String,
}

pub async fn get_me(
    database_pool: &DatabasePool,
    identity: &Identity,
) -> ServiceResult<AccountOutput> {
    let entity = identity.require_account(Permission::Default)?;
    let entity = entity.joined(database_pool).await?;
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

    if let Some(nfc_card_id) = input.nfc_card_id {
        if let Some(nfc_card_secret) = input.nfc_card_secret {
            let login_result = authenticate_nfc_mifare_desfire_login(
                database_pool,
                &nfc_card_id,
                &nfc_card_secret,
            )
            .await;
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
                Err(_) => Err(ServiceError::Unauthorized("invalid username/password")),
            }
        }
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

pub async fn set_account_password(
    database_pool: &DatabasePool,
    identity: &Identity,
    account_id: Uuid,
    old_password: Option<&str>,
    new_password: &str,
) -> ServiceResult<()> {
    let own_account = identity.require_account(Permission::Default)?;

    if account_id == own_account.id || identity.require_account(Permission::Admin).is_ok() {
        let account = Account::get(database_pool, account_id).await?;

        let has_password = authentication_password::has_password(database_pool, &account).await?;

        if has_password {
            if !authentication_password::verify_password(
                database_pool,
                &account,
                old_password.unwrap_or(""),
            )
            .await?
            {
                return Err(ServiceError::Unauthorized("Old password does not match!"));
            }
            authentication_password::register(database_pool, &account, new_password).await?;
        } else {
            if old_password.is_some() {
                return Err(ServiceError::Unauthorized("Old password does not match!"));
            }
            authentication_password::register(database_pool, &account, new_password).await?;
        }
    } else {
        return Err(ServiceError::Unauthorized(""));
    }

    Ok(())
}

pub async fn delete_account_password(
    database_pool: &DatabasePool,
    identity: &Identity,
    account_id: Uuid,
) -> ServiceResult<()> {
    let own_account = identity.require_account(Permission::Default)?;

    if account_id == own_account.id || identity.require_account(Permission::Admin).is_ok() {
        let account = Account::get(database_pool, account_id).await?;
        authentication_password::remove(database_pool, &account).await?;
    } else {
        return Err(ServiceError::Unauthorized(""));
    }

    Ok(())
}
