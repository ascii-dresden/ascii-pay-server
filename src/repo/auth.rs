use crate::{
    identity_service::{Identity, IdentityMut, IdentityRequire},
    model::{
        authentication_password,
        session::{get_onetime_session, Session},
        Permission,
    },
    utils::{DatabaseConnection, RedisConnection, ServiceError, ServiceResult},
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

pub fn login(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    identity: &Identity,
    input: LoginInput,
) -> ServiceResult<LoginOutput> {
    if let Some(account_access_token) = input.account_access_token {
        let login_result = get_onetime_session(database_conn, redis_conn, &account_access_token);

        return match login_result {
            Ok(account) => {
                identity.store(database_conn, redis_conn, account.id)?;

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

    let login_result = authentication_password::get(database_conn, &username, &password);
    match login_result {
        Ok(account) => {
            identity.store(database_conn, redis_conn, account.id)?;

            let token = identity.require_auth_token()?;
            Ok(LoginOutput {
                authorization: format!("Bearer {}", &token),
                token,
            })
        }
        Err(_) => Err(ServiceError::Unauthorized("invalid username/password")),
    }
}

pub fn login_mut(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    identity: &IdentityMut,
    input: LoginInput,
) -> ServiceResult<LoginOutput> {
    if let Some(account_access_token) = input.account_access_token {
        let login_result = get_onetime_session(database_conn, redis_conn, &account_access_token);

        return match login_result {
            Ok(account) => {
                identity.store(database_conn, redis_conn, account.id)?;

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

    let login_result = authentication_password::get(database_conn, &username, &password);
    match login_result {
        Ok(account) => {
            identity.store(database_conn, redis_conn, account.id)?;

            let token = identity.require_auth_token()?;
            Ok(LoginOutput {
                authorization: format!("Bearer {}", &token),
                token,
            })
        }
        Err(_) => Err(ServiceError::Unauthorized("invalid username/password")),
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
