lazy_static::lazy_static! {
    /// Host name of the application. The web server only listens to request with a matching host name.
    ///
    /// Field name: `HOST`
    pub static ref API_HOST: String = std::env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());

    /// The application port.
    ///
    /// Field name: `PORT`
    pub static ref API_PORT: String = std::env::var("API_PORT").unwrap_or_else(|_| "3000".to_owned());

    /// Database connection string.
    ///
    /// Field name: `DATABASE_URI`
    pub static ref DATABASE_URL: String = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://ascii:ascii@localhost:5432/ascii-pay".to_owned());

    /// Domain name for links and cookies.
    ///
    /// Field name: `DOMAIN_NAME`
    pub static ref DOMAIN_NAME: String = std::env::var("DOMAIN_NAME").unwrap_or_else(|_| "https://pay.ascii.coffee".to_owned());

    /// Source address for mails, eg:
    /// payments@ascii.coffee
    ///
    /// Field name: `MAIL_SENDER`
    pub static ref MAIL_SENDER: String = std::env::var("MAIL_SENDER").unwrap_or_else(|_| "ascii-pay@ascii.coffee".to_owned());

    /// Sender name for mails, eg:
    /// "Ascii Pay Service"
    ///
    /// Field name: `MAIL_SENDER_NAME`
    pub static ref MAIL_SENDER_NAME: String = std::env::var("MAIL_SENDER_NAME").unwrap_or_else(|_| "ascii-pay system".to_owned());

    /// Mail server url. Server can be set to test mode if url ends with `.local` eg: `mail.example.local`.
    /// Otherwise the url is used to verify tls certificates.
    ///
    /// Field name: `MAIL_SERVER`
    pub static ref MAIL_SERVER: String = std::env::var("MAIL_SERVER").unwrap_or_else(|_| "mail.example.local".to_owned());

    /// Login username for mail server.
    ///
    /// Field name: `MAIL_USER`
    pub static ref MAIL_USER: String = std::env::var("MAIL_USER").unwrap_or_else(|_| "ascii".to_owned());

    /// Login password for mail server.
    ///
    /// Field name: `MAIL_PASS`
    pub static ref MAIL_PASS: String = std::env::var("MAIL_PASS").unwrap_or_else(|_| "ascii".to_owned());
}
