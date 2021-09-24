use diesel::prelude::*;
use log::{error, info};
use std::io::Cursor;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use wallet_pass::{template, Pass};

use crate::model::schema::{apple_wallet_pass, apple_wallet_registration};
use crate::utils::{
    env, generate_uuid, ApplePushNotificationService, DatabaseConnection, ServiceError,
    ServiceResult,
};

use super::Account;

pub fn get_current_time() -> i32 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    since_the_epoch.as_secs() as i32
}

/// Represent a wallet pass
#[derive(
    Debug, Queryable, Insertable, AsChangeset, PartialEq, Eq, Hash, Serialize, Deserialize, Clone,
)]
#[table_name = "apple_wallet_pass"]
struct AppleWalletPass {
    pub serial_number: Uuid,
    pub authentication_token: Uuid,
    pub qr_code: String,
    pub pass_type_id: String,
    pub updated_at: i32,
}

/// Represent a wallet registration
#[derive(
    Debug, Queryable, Insertable, AsChangeset, PartialEq, Eq, Hash, Serialize, Deserialize, Clone,
)]
#[table_name = "apple_wallet_registration"]
struct AppleWalletRegistration {
    pub device_id: String,
    pub serial_number: Uuid,
    pub push_token: String,
    pub pass_type_id: String,
}

pub fn check_pass_authorization(
    database_conn: &DatabaseConnection,
    serial_number: Uuid,
    authentication_token: Uuid,
) -> ServiceResult<bool> {
    use crate::model::schema::apple_wallet_pass::dsl;
    let results = dsl::apple_wallet_pass
        .filter(dsl::serial_number.eq(serial_number))
        .filter(dsl::authentication_token.eq(authentication_token))
        .load::<AppleWalletPass>(database_conn)?;

    Ok(!results.is_empty())
}

pub fn is_pass_registered_on_device(
    database_conn: &DatabaseConnection,
    device_id: &str,
    serial_number: Uuid,
) -> ServiceResult<bool> {
    use crate::model::schema::apple_wallet_registration::dsl;
    let results = dsl::apple_wallet_registration
        .filter(dsl::device_id.eq(device_id))
        .filter(dsl::serial_number.eq(serial_number))
        .load::<AppleWalletRegistration>(database_conn)?;

    Ok(!results.is_empty())
}

pub fn register_pass_on_device(
    database_conn: &DatabaseConnection,
    device_id: &str,
    serial_number: Uuid,
    pass_type_id: &str,
    push_token: &str,
) -> ServiceResult<()> {
    use crate::model::schema::apple_wallet_registration::dsl;

    let r = AppleWalletRegistration {
        device_id: device_id.to_owned(),
        serial_number: serial_number.to_owned(),
        push_token: push_token.to_owned(),
        pass_type_id: pass_type_id.to_owned(),
    };

    diesel::insert_into(dsl::apple_wallet_registration)
        .values(&r)
        .execute(database_conn)?;
    Ok(())
}

pub fn unregister_pass_on_device(
    database_conn: &DatabaseConnection,
    device_id: &str,
    serial_number: Uuid,
) -> ServiceResult<()> {
    use crate::model::schema::apple_wallet_registration::dsl;

    diesel::delete(
        dsl::apple_wallet_registration
            .filter(dsl::device_id.eq(device_id))
            .filter(dsl::serial_number.eq(serial_number)),
    )
    .execute(database_conn)?;
    Ok(())
}

pub fn is_device_registered(
    database_conn: &DatabaseConnection,
    device_id: &str,
) -> ServiceResult<bool> {
    use crate::model::schema::apple_wallet_registration::dsl;

    let results = dsl::apple_wallet_registration
        .filter(dsl::device_id.eq(device_id))
        .load::<AppleWalletRegistration>(database_conn)?;

    Ok(!results.is_empty())
}

pub fn list_passes_for_device(
    database_conn: &DatabaseConnection,
    device_id: &str,
    pass_type_id: &str,
) -> ServiceResult<Vec<Uuid>> {
    use crate::model::schema::apple_wallet_registration::dsl;
    let results = dsl::apple_wallet_registration
        .filter(dsl::device_id.eq(device_id))
        .filter(dsl::pass_type_id.eq(pass_type_id))
        .load::<AppleWalletRegistration>(database_conn)?;

    Ok(results.into_iter().map(|r| r.serial_number).collect())
}

pub fn get_pass_updated_at(
    database_conn: &DatabaseConnection,
    serial_number: Uuid,
) -> ServiceResult<i32> {
    use crate::model::schema::apple_wallet_pass::dsl;
    let mut results = dsl::apple_wallet_pass
        .filter(dsl::serial_number.eq(serial_number))
        .load::<AppleWalletPass>(database_conn)?;

    if results.len() != 1 {
        return Err(ServiceError::NotFound);
    }

    Ok(results.pop().ok_or(ServiceError::NoneError)?.updated_at)
}

pub fn get_by_qr_code(database_conn: &DatabaseConnection, qr_code: &str) -> ServiceResult<Uuid> {
    use crate::model::schema::apple_wallet_pass::dsl;
    let mut results = dsl::apple_wallet_pass
        .filter(dsl::qr_code.eq(qr_code))
        .load::<AppleWalletPass>(database_conn)?;

    if results.len() != 1 {
        return Err(ServiceError::NotFound);
    }

    Ok(results.pop().ok_or(ServiceError::NoneError)?.serial_number)
}

pub fn create_pass(
    database_conn: &DatabaseConnection,
    account: &Account,
) -> ServiceResult<Vec<u8>> {
    use crate::model::schema::apple_wallet_pass::dsl;

    let mut results = dsl::apple_wallet_pass
        .filter(dsl::serial_number.eq(&account.id))
        .load::<AppleWalletPass>(database_conn)?;

    let db_pass = match results.len() {
        0 => {
            let qr_code = format!(
                "{}-{}",
                account
                    .id
                    .to_hyphenated()
                    .encode_upper(&mut Uuid::encode_buffer()),
                generate_uuid()
                    .to_hyphenated()
                    .encode_upper(&mut Uuid::encode_buffer())
            );

            let db_pass = AppleWalletPass {
                serial_number: account.id,
                authentication_token: generate_uuid(),
                qr_code,
                pass_type_id: env::APPLE_WALLET_PASS_TYPE_IDENTIFIER.as_str().to_owned(),
                updated_at: get_current_time(),
            };

            diesel::insert_into(dsl::apple_wallet_pass)
                .values(&db_pass)
                .execute(database_conn)?;
            db_pass
        }
        1 => results.pop().ok_or(ServiceError::NoneError)?,
        _ => {
            return Err(ServiceError::NotFound);
        }
    };

    // Load template
    let pass_path = Path::new(env::APPLE_WALLET_TEMPLATE.as_str());
    let mut pass = Pass::from_path(pass_path)?;

    // Set general fields
    pass.pass_type_identifier(env::APPLE_WALLET_PASS_TYPE_IDENTIFIER.as_str());
    pass.team_identifier(env::APPLE_WALLET_TEAM_IDENTIFIER.as_str());
    pass.web_service_url(env::APPLE_WALLET_SERVICE_URL.as_str());

    // Set account specific fields
    pass.serial_number(
        db_pass
            .serial_number
            .to_hyphenated()
            .encode_upper(&mut Uuid::encode_buffer()),
    );
    pass.authentication_token(
        db_pass
            .authentication_token
            .to_hyphenated()
            .encode_upper(&mut Uuid::encode_buffer()),
    );

    pass.add_barcodes(template::Barcode::new(
        template::BarcodeFormat::PkBarcodeFormatQr,
        &db_pass.qr_code,
        "iso-8859-1",
    ));

    let mut store_card = template::Details::new();

    let mut field = template::Field::new_f64("balance", (account.credit as f64) / 100.0);
    field.label("balance");
    field.currency_code("EUR");
    field.change_message("balance_updated");
    store_card.add_primary_field(field);

    let mut field = template::Field::new_string("account_name", &account.name);
    field.label("account_name");
    store_card.add_secondary_field(field);

    if let Some(account_number) = &account.account_number {
        let mut field = template::Field::new_string("account_number", account_number);
        field.label("account_number");
        store_card.add_secondary_field(field);

        let mut field = template::Field::new_string(
            "account_login",
            &format!("https://pay.ascii.coffee?code={}", &account_number),
        );
        field.label("account_login");
        store_card.add_back_field(field);
    }

    pass.store_card(store_card);

    // Export
    let vec = Vec::<u8>::with_capacity(100_000);
    let cursor = pass.export(
        Path::new(env::APPLE_WALLET_PASS_CERTIFICATE.as_str()),
        env::APPLE_WALLET_PASS_CERTIFICATE_PASSWORD.as_str(),
        Path::new(env::APPLE_WALLET_WWDR_CERTIFICATE.as_str()),
        Cursor::new(vec),
    )?;

    Ok(cursor.into_inner())
}

pub fn delete_pass(database_conn: &DatabaseConnection, account_id: Uuid) -> ServiceResult<()> {
    use crate::model::schema::apple_wallet_pass::dsl as dsl_pass;
    use crate::model::schema::apple_wallet_registration::dsl as dsl_registration;

    diesel::delete(
        dsl_registration::apple_wallet_registration
            .filter(dsl_registration::serial_number.eq(account_id)),
    )
    .execute(database_conn)?;

    diesel::delete(dsl_pass::apple_wallet_pass.filter(dsl_pass::serial_number.eq(account_id)))
        .execute(database_conn)?;

    Ok(())
}

pub fn set_pass_updated_at(
    database_conn: &DatabaseConnection,
    serial_number: Uuid,
) -> ServiceResult<()> {
    use crate::model::schema::apple_wallet_pass::dsl;

    diesel::update(
        dsl::apple_wallet_pass.filter(apple_wallet_pass::serial_number.eq(serial_number)),
    )
    .set(dsl::updated_at.eq(get_current_time()))
    .execute(database_conn)?;

    Ok(())
}

pub async fn send_update_notification(
    database_conn: &DatabaseConnection,
    account_id: Uuid,
) -> ServiceResult<()> {
    use crate::model::schema::apple_wallet_registration::dsl;

    set_pass_updated_at(database_conn, account_id)?;

    let results = dsl::apple_wallet_registration
        .filter(dsl::serial_number.eq(account_id))
        .load::<AppleWalletRegistration>(database_conn)?;

    info!("Send APNS message for account: {:?}", account_id);

    let mut unregister_vec = Vec::<String>::new();

    let apns = ApplePushNotificationService::new()?;
    for registration in results {
        let response_code = match apns.send(&registration.push_token).await {
            Ok(response_code) => response_code,
            Err(e) => {
                error!("Error while communicating with APNS: {:?}", e);
                continue;
            }
        };

        let unregister = match response_code {
            200 => false,
            410 => true,
            _ => {
                return Err(ServiceError::InternalServerError(
                    "APNS returned illegal status code!",
                    format!("Status code: {}", response_code),
                ))
            }
        };

        if unregister {
            unregister_vec.push(registration.device_id.clone());
        }
    }

    for device_id in unregister_vec {
        if let Err(e) = unregister_pass_on_device(database_conn, &device_id, account_id) {
            error!(
                "Cannot unregister device {} as APNS requested: {:?}",
                &device_id, e
            );
        }
    }

    Ok(())
}
