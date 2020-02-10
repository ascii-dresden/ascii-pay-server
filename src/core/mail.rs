use crate::core::authentication_password::InvitationLink;
use crate::core::{Account, ServiceError, ServiceResult};
use lettre::smtp::authentication::Credentials;
use lettre::{SendableEmail, SmtpClient, Transport};
use lettre_email::EmailBuilder;

struct MailCredentials {
    pub sender: String,
    pub sender_name: String,
    pub server: String,
    pub user: String,
    pub pass: String,
}

impl MailCredentials {
    fn load_from_environment() -> Self {
        MailCredentials {
            sender: std::env::var("MAIL_SENDER").expect("MAIL_SENDER must be set."),
            sender_name: std::env::var("MAIL_SENDER_NAME").expect("MAIL_SENDER_NAME must be set."),
            server: std::env::var("MAIL_URL").expect("MAIL_URL must be set"),
            user: std::env::var("MAIL_USER").expect("MAIL_USER must be set"),
            pass: std::env::var("MAIL_PASSWORD").expect("MAIL_PASSWORD must be set"),
        }
    }
}

fn send_standard_mail(account: &Account, subj: &str, message: String) -> ServiceResult<()> {
    let credentials = MailCredentials::load_from_environment();

    let mail_address = if let Some(m) = account.mail.as_ref() {
        m
    } else {
        return Err(ServiceError::InternalServerError(
            "No Mail address provided",
            String::from("A mail sending context was called, but no mail address was provided."),
        ));
    };

    let email = EmailBuilder::new()
        // Addresses can be specified by the tuple (email, alias)
        .to((
            mail_address,
            &account.name,
        ))
        .from((credentials.sender, credentials.sender_name))
        .subject(subj)
        .text(message)
        .build()?;

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

pub fn send_invitation_link(account: &Account, invite: &InvitationLink) -> ServiceResult<()> {
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

    send_standard_mail(
        account,
        "[ascii pay] You have been invited to the ascii-pay service",
        mail_text,
    )
}

/// Send a generated monthly report to the user
pub fn send_report_mail(account: &Account, subject: String, report: String) -> ServiceResult<()> {
    send_standard_mail(account, &subject, report)
}

// TODO: Needs a route!
/// Sends a test mail to the given receiver.
pub fn send_test_mail(receiver: String) -> ServiceResult<()> {
    let credentials = MailCredentials::load_from_environment();

    let mail = EmailBuilder::new()
        .to(receiver)
        .from(credentials.sender)
        .subject("[ascii pay] Test Mail")
        .text("This is a test mail to verify that the mailing system works.")
        .build()?;

    let mut mailer = SmtpClient::new_simple(&credentials.server)?
        .credentials(Credentials::new(credentials.user, credentials.pass))
        .transport();

    // Send the email
    let _ = mailer.send(mail.into())?;

    Ok(())
}
