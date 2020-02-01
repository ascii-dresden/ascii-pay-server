use crate::core::authentication_password::InvitationLink;
use crate::core::{Account, ServiceResult};
use lettre::smtp::authentication::Credentials;
use lettre::{SendableEmail, SmtpClient, Transport};
use lettre_email::EmailBuilder;

struct MailCredentials {
    pub sender: String,
    pub server: String,
    pub user: String,
    pub pass: String,
}

impl MailCredentials {
    fn load_from_environment() -> Self {
        MailCredentials {
            sender: std::env::var("MAIL_SENDER").expect("MAIL_SENDER must be set."),
            server: std::env::var("MAIL_URL").expect("MAIL_URL must be set"),
            user: std::env::var("MAIL_USER").expect("MAIL_USER must be set"),
            pass: std::env::var("MAIL_PASSWORD").expect("MAIL_PASSWORD must be set"),
        }
    }
}

// TODO: Adjust error type

pub fn send_invitation_link(account: &Account, invite: &InvitationLink) -> ServiceResult<()> {
    let credentials = MailCredentials::load_from_environment();

    let mail_text = format!("Hello {user},

you have been invited to create an account in the ascii-pay system. You can use the following link to commence account creation.
Please note that your invitation will expire at {date}.

{link}

The Ascii Pay System

----
This mail has been automatically generated. Please do not reply.",
        user = account.name,
        date = invite.valid_until.format("%d.%m.%Y %H:%M"),
        link = invite);

    let email = EmailBuilder::new()
        // Addresses can be specified by the tuple (email, alias)
        .to((
            account
                .mail
                .as_ref()
                .expect("No mail address submitted to invite send function"),
            &account.name,
        ))
        // ... or by an address only
        .from(credentials.sender)
        .subject("[ascii pay] You have been invited to the ascii-pay service")
        .text(mail_text)
        .build()
        .unwrap();

    if credentials.server.ends_with(".local") {
        // dump the mail to the log
        let m: SendableEmail = email.into();
        println!(
            "{}",
            m.message_to_string()
                .expect("This was unrealistic to happen")
        );
    } else {
        // Open a smtp connection
        let mut mailer = SmtpClient::new_simple(&credentials.server)?
            .credentials(Credentials::new(credentials.user, credentials.pass))
            .transport();

        // Send the email
        let _ = mailer.send(email.into())?;
    }

    Ok(())
}

/// Sends a test mail to the given receiver.
pub fn send_test_mail(receiver: String) -> ServiceResult<()> {
    let credentials = MailCredentials::load_from_environment();

    let mail = EmailBuilder::new()
        .to(receiver)
        .from(credentials.sender)
        .subject("[ascii pay] Test Mail")
        .text("This is a test mail to verify that the miling system works.")
        .build()
        .unwrap();

    let mut mailer = SmtpClient::new_simple(&credentials.server)?
        .credentials(Credentials::new(credentials.user, credentials.pass))
        .transport();

    // Send the email
    let _ = mailer.send(mail.into())?;

    Ok(())
}
