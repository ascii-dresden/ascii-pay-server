use std::str::FromStr;

use chrono::{DateTime, FixedOffset, Utc};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use log::info;

use crate::{
    env,
    error::{ServiceError, ServiceResult},
    models::{Account, CoinType, Transaction},
};

pub async fn send_standard_mail(
    account: &Account,
    subj: &str,
    message: String,
) -> ServiceResult<()> {
    if account.email.is_empty() {
        return Err(ServiceError::InternalServerError(String::from(
            "A mail sending context was called, but no mail address was provided.",
        )));
    };

    let email = Message::builder()
        // Addresses can be specified by the tuple (email, alias)
        .to(Mailbox::new(
            Some(account.name.clone()),
            Address::from_str(&account.email).unwrap(),
        ))
        .from(Mailbox::new(
            Some(env::MAIL_SENDER_NAME.clone()),
            Address::from_str(env::MAIL_SENDER.as_str()).unwrap(),
        ))
        .subject(subj)
        .header(ContentType::TEXT_PLAIN)
        .body(message)?;

    if env::MAIL_SERVER.as_str().ends_with(".local") {
        let bytes = email.formatted();
        let content = String::from_utf8(bytes).unwrap();
        info!("{content}");
    } else {
        let credentials = Credentials::new(env::MAIL_USER.clone(), env::MAIL_PASS.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&env::MAIL_SERVER)
            .unwrap()
            .credentials(credentials)
            .build();

        mailer.send(email).await?;
    }

    Ok(())
}

pub async fn send_invitation_link(
    account: &Account,
    token: &str,
    valid_until: &DateTime<Utc>,
) -> ServiceResult<()> {
    let timezone = FixedOffset::east_opt(60 * 60).unwrap();

    let mail_text = format!("Hello {user},

you have been invited to create an account in the ascii-pay system. You can use the following link to commence account creation.
Please note that your invitation will expire at {date}.

{domain}/reset-password?token={token}

The ascii-pay System

----
This mail has been automatically generated. Please do not reply.",
        user = account.name,
        date = valid_until.with_timezone(&timezone).format("%d.%m.%Y %H:%M"),
        domain = env::DOMAIN_NAME.as_str(),
        token = token);

    send_standard_mail(
        account,
        "[ascii-pay] You have been invited to the ascii-pay service",
        mail_text,
    )
    .await
}

pub async fn send_monthly_report(
    account: &Account,
    transactions: &[Transaction],
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> ServiceResult<()> {
    let timezone = FixedOffset::east_opt(60 * 60).unwrap();

    fn get_type_amounts(t: CoinType, items: &[Transaction]) -> Vec<i32> {
        items
            .iter()
            .flat_map(|item| {
                item.items
                    .iter()
                    .map(|item| *item.effective_price.0.get(&t).unwrap_or(&0))
            })
            .collect()
    }

    let total_up_cents: i32 = get_type_amounts(CoinType::Cent, transactions)
        .into_iter()
        .filter(|i| *i > 0)
        .sum();

    let mail_text = format!(
        "Hello {user},

here is your monthly report for {start_date} to {end_date}:

You spend:  {total_up_cents: >7.2} €

The ascii-pay System

----
This mail has been automatically generated. Please do not reply.",
        user = account.name,
        start_date = start_date.with_timezone(&timezone).format("%d.%m.%Y"),
        end_date = end_date.with_timezone(&timezone).format("%d.%m.%Y"),
        total_up_cents = f64::from(total_up_cents) / 100.0,
    );

    send_standard_mail(account, "[ascii-pay] Monthly report", mail_text).await
}
