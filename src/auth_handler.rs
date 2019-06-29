use actix::{Handler, Message};
use actix_web::{dev::Payload, Error, HttpRequest, FromRequest};
use actix_identity::Identity;
use bcrypt::verify;
use diesel::prelude::*;

use crate::errors::ServiceError;
use crate::models::{DbExecutor, SlimUser, User, Account};
use crate::utils::decode_token;

#[derive(Debug, Deserialize)]
pub struct AuthData {
    pub user: String,
    pub password: String,
}

impl Message for AuthData {
    type Result = Result<SlimUser, ServiceError>;
}

impl Handler<AuthData> for DbExecutor {
    type Result = Result<SlimUser, ServiceError>;
    fn handle(&mut self, msg: AuthData, _: &mut Self::Context) -> Self::Result {
        //use crate::schema::users::dsl::{mail, users, account_id};
        //use crate::schema::accounts::dsl::{id, display, accounts};
        use crate::schema::*;

        let conn: &SqliteConnection = &self.0.get().unwrap();

        let mut items = accounts::table
            .inner_join(
                users::table.on(users::account_id.eq(accounts::id))
            )
            .filter(users::mail.eq(&msg.user).or(accounts::display.eq(&msg.user)))
            .load::<(Account, User)>(conn)?;

        if let Some((_account, user)) = items.pop() {
            match verify(&msg.password, &user.password) {
                Ok(matching) => {
                    if matching {
                        return Ok(user.into());
                    }
                }
                Err(_) => (),
            }
        }
        Err(ServiceError::BadRequest(
            "Username and Password don't match".into(),
        ))
    }
}

// we need the same data
// simple aliasing makes the intentions clear and its more readable
pub type LoggedUser = SlimUser;

impl FromRequest for LoggedUser {
    type Error = Error;
    type Future = Result<LoggedUser, Error>;
    type Config = ();

    fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
        if let Some(identity) = Identity::from_request(req, pl)?.identity() {
            let user: SlimUser = decode_token(&identity)?;
            return Ok(user as LoggedUser);
        }
        Err(ServiceError::Unauthorized.into())
    }
}
