lazy_static::lazy_static! {
    /// Host name of the application. The web server only listens to request with a matching host name.
    ///
    /// Field name: `HOST`
    pub static ref HOST: String = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_owned());

    /// The application port.
    ///
    /// Field name: `PORT`
    pub static ref HTTP_PORT: u16 = std::env::var("HTTP_PORT")
        .unwrap_or_else(|_| "".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    /// The application port.
    ///
    /// Field name: `PORT`
    pub static ref GRPC_PORT: u16 = std::env::var("GRPC_PORT")
        .unwrap_or_else(|_| "".to_string())
        .parse::<u16>()
        .unwrap_or(8081);

    /// Domain string for cookies. Cookies will be valid for this domain name.
    ///
    /// Field name: `DOMAIN`
    pub static ref DOMAIN: String = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_owned());

    /// Base url for server generated urls. This is the base reference for eg. password invitation links.
    ///
    /// Field name: `BASE_URL`
    pub static ref BASE_URL: String = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_owned());

    /// Database connection string.
    ///
    /// Field name: `DATABASE_URI`
    pub static ref DATABASE_URI: String = std::env::var("DATABASE_URI").expect("DATABASE_URI must be set");

    /// Redis connection string.
    ///
    /// Field name: `REDIS_URI`
    pub static ref REDIS_URI: String = std::env::var("REDIS_URI").expect("REDIS_URI must be set");

    /// Product storage path.
    ///
    /// Field name: `PRODUCT_STORAGE`
    pub static ref PRODUCT_STORAGE: String = std::env::var("PRODUCT_STORAGE").expect("PRODUCT_STORAGE must be set");

    /// Salt for password hashing.
    ///
    /// Field name: `PASSWORD_SALT`
    pub static ref PASSWORD_SALT: String = std::env::var("PASSWORD_SALT").unwrap_or_else(|_| "0123012301230123".repeat(8));

    /// Encryption key for authentication cookies.
    ///
    /// Field name: `COOKIE_ENCRYPTION_KEY`
    pub static ref COOKIE_ENCRYPTION_KEY: String = std::env::var("COOKIE_ENCRYPTION_KEY").unwrap_or_else(|_| "0123".repeat(8));

    /// Access key for api routes.
    ///
    /// Field name: `API_ACCESS_KEY`
    pub static ref API_ACCESS_KEY: String = std::env::var("API_ACCESS_KEY").unwrap_or_else(|_| "true".to_owned());

    /// Header secret to access cron urls.
    ///
    /// Field name: `CRON_SECRET`
    pub static ref CRON_SECRET: String = std::env::var("CRON_SECRET").expect("CRON_SECRET must be set.");

    /// Source address for mails, eg:
    /// payments@ascii.coffee
    ///
    /// Field name: `MAIL_SENDER`
    pub static ref MAIL_SENDER: String = std::env::var("MAIL_SENDER").expect("MAIL_SENDER must be set.");

    /// Sender name for mails, eg:
    /// "Ascii Pay Service"
    ///
    /// Field name: `MAIL_SENDER_NAME`
    pub static ref MAIL_SENDER_NAME: String = std::env::var("MAIL_SENDER_NAME").expect("MAIL_SENDER_NAME must be set.");

    /// Mail server url. Server can be set to test mode if url ends with `.local` eg: `mail.example.local`.
    /// Otherwise the url is used to verify tls certificates.
    ///
    /// Field name: `MAIL_URL`
    pub static ref MAIL_SERVER: String = std::env::var("MAIL_URL").expect("MAIL_URL must be set");

    /// Login username for mail server.
    ///
    /// Field name: `MAIL_USER`
    pub static ref MAIL_USER: String = std::env::var("MAIL_USER").expect("MAIL_USER must be set");

    /// Login password for mail server.
    ///
    /// Field name: `MAIL_PASSWORD`
    pub static ref MAIL_PASS: String = std::env::var("MAIL_PASSWORD").expect("MAIL_PASSWORD must be set");

    /// Url to this server. This can differ from the BASE_URL if the apple wallet service is not mounted at root.
    ///
    /// Field name: `APPLE_WALLET_SERVICE_URL`
    pub static ref APPLE_WALLET_SERVICE_URL: String = std::env::var("APPLE_WALLET_SERVICE_URL").unwrap_or_else(|_| "https://pay.ascii.coffee/".to_owned());

    /// Path to the apple wallet template.
    ///
    /// Field name: `APPLE_WALLET_TEMPLATE`
    pub static ref APPLE_WALLET_TEMPLATE: String = std::env::var("APPLE_WALLET_TEMPLATE").unwrap_or_else(|_| "./AsciiPayCard.pass".to_owned());

    /// Path to the apple wallet apns certificate.
    ///
    /// Field name: `APPLE_WALLET_APNS_CERTIFICATE`
    pub static ref APPLE_WALLET_APNS_CERTIFICATE: String = std::env::var("APPLE_WALLET_APNS_CERTIFICATE").unwrap_or_else(|_| "./certificates/apple-apns.pem".to_owned());

    /// Path to the apple wallet wwdr certificate.
    ///
    /// Field name: `APPLE_WALLET_WWDR_CERTIFICATE`
    pub static ref APPLE_WALLET_WWDR_CERTIFICATE: String = std::env::var("APPLE_WALLET_WWDR_CERTIFICATE").unwrap_or_else(|_| "./certificates/apple-wwdr.pem".to_owned());

    /// Path to the apple wallet pass certificate.
    ///
    /// Field name: `APPLE_WALLET_PASS_CERTIFICATE`
    pub static ref APPLE_WALLET_PASS_CERTIFICATE: String = std::env::var("APPLE_WALLET_PASS_CERTIFICATE").unwrap_or_else(|_| "./certificates/apple-pass.p12".to_owned());

    /// Password for apple wallet pass certificate.
    ///
    /// Field name: `APPLE_WALLET_PASS_CERTIFICATE_PASSWORD`
    pub static ref APPLE_WALLET_PASS_CERTIFICATE_PASSWORD: String = std::env::var("APPLE_WALLET_PASS_CERTIFICATE_PASSWORD").unwrap_or_else(|_| "ascii".to_owned());

    /// The pass type identifier as registered by apple.
    ///
    /// Field name: `APPLE_WALLET_PASS_TYPE_IDENTIFIER`
    pub static ref APPLE_WALLET_PASS_TYPE_IDENTIFIER: String = std::env::var("APPLE_WALLET_PASS_TYPE_IDENTIFIER").unwrap_or_else(|_| "pass.coffee.ascii.pay".to_owned());

    /// The team identifier that was used to register the pass type identifier by apple.
    ///
    /// Field name: `APPLE_WALLET_TEAM_IDENTIFIER`
    pub static ref APPLE_WALLET_TEAM_IDENTIFIER: String = std::env::var("APPLE_WALLET_TEAM_IDENTIFIER").unwrap_or_else(|_| "QVU8H45PQ5".to_owned());
}
