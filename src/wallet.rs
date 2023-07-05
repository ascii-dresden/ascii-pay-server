use log::{error, info};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::io::Cursor;
use std::time::{SystemTime, UNIX_EPOCH};
use wallet_pass::{template, Pass};

use crate::apns::ApplePushNotificationService;
use crate::database::DatabaseConnection;
use crate::env;
use crate::error::{ServiceError, ServiceResult};
use crate::models::{Account, AppleWalletPass, CoinType};

pub fn get_current_time() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    since_the_epoch.as_secs()
}

pub fn generate_random_string(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn create_pass_binary(
    account: &Account,
    wallet_pass: &AppleWalletPass,
) -> ServiceResult<Vec<u8>> {
    // Load template
    let mut pass = Pass::from_path(env::APPLE_WALLET_TEMPLATE.as_str())?;

    // Set general fields
    pass.pass_type_identifier(env::APPLE_WALLET_PASS_TYPE_IDENTIFIER.as_str());
    pass.team_identifier(env::APPLE_WALLET_TEAM_IDENTIFIER.as_str());
    pass.web_service_url(env::APPLE_WALLET_SERVICE_URL.as_str());

    // Set account specific fields
    pass.serial_number(&wallet_pass.account_id.to_string());
    pass.authentication_token(&wallet_pass.authentication_token);

    pass.add_barcodes(template::Barcode::new(
        template::BarcodeFormat::PkBarcodeFormatQr,
        &wallet_pass.qr_code,
        "iso-8859-1",
    ));

    let mut store_card = template::Details::new();

    let cents = account.balance.0.get(&CoinType::Cent).copied().unwrap_or(0);
    let mut field = template::Field::new_f64("balance", (cents as f64) / 100.0);
    field.label("balance");
    field.currency_code("EUR");
    field.change_message("balance_updated");
    store_card.add_primary_field(field);

    let bottle_stamps: i32 = account.balance.0.get(&CoinType::BottleStamp).copied().unwrap_or(0);
    let mut field = template::Field::new_f64("account_bottle_stamps",  bottle_stamps as f64);
    field.label("account_bottle_stamps");
    field.change_message("bottle_stamps_updated");
    store_card.add_secondary_field(field);

    let coffee_stamps = account.balance.0.get(&CoinType::CoffeeStamp).copied().unwrap_or(0);
    let mut field = template::Field::new_f64("account_coffee_stamps",  coffee_stamps as f64);
    field.label("account_coffee_stamps");
    field.change_message("coffee_stamps_updated");
    store_card.add_secondary_field(field);

    let mut field = template::Field::new_string("account_name", &account.name);
    field.label("account_name");
    store_card.add_back_field(field);

    pass.store_card(store_card);

    // Export
    let vec = Vec::<u8>::new();
    let cursor = pass.export(
        env::APPLE_WALLET_PASS_CERTIFICATE.as_str(),
        env::APPLE_WALLET_PASS_CERTIFICATE_PASSWORD.as_str(),
        env::APPLE_WALLET_WWDR_CERTIFICATE.as_str(),
        Cursor::new(vec),
    )?;

    Ok(cursor.into_inner())
}

pub async fn send_update_notification(
    db: &mut DatabaseConnection,
    account_id: u64,
) -> ServiceResult<()> {
    let pass = db
        .get_apple_wallet_pass(account_id, &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER)
        .await?;

    if let Some(mut pass) = pass {
        pass.updated_at = get_current_time();
        db.store_apple_wallet_pass(pass).await?;
    } else {
        return Ok(());
    }

    let registrations = db
        .list_apple_wallet_registration(account_id, &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER)
        .await?;

    info!("Send APNS message for account: {:?}", account_id);

    let mut unregister_vec = Vec::<String>::new();

    let apns = ApplePushNotificationService::new()?;
    for registration in registrations {
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
                return Err(ServiceError::InternalServerError(format!(
                    "APNS returned illegal status code: {}",
                    response_code
                )))
            }
        };

        if unregister {
            unregister_vec.push(registration.device_id.clone());
        }
    }

    for device_id in unregister_vec {
        if let Err(e) = db
            .delete_apple_wallet_registration(
                account_id,
                &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER,
                &device_id,
            )
            .await
        {
            error!(
                "Cannot unregister device {} as APNS requested: {:?}",
                &device_id, e
            );
        }
    }

    Ok(())
}
