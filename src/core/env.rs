lazy_static::lazy_static! {
    /// Host name of the application. The web server only listens to request with a matching host name.
    /// 
    /// Field name: `HOST`
    pub static ref HOST: String = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_owned());
}

lazy_static::lazy_static! {
    /// The application port.
    /// 
    /// Field name: `PORT`
    pub static ref PORT: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
}

lazy_static::lazy_static! {
    /// Domain string for cookies. Cookies will be valid for this domain name.
    /// 
    /// Field name: `DOMAIN`
    pub static ref DOMAIN: String = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_owned());
}

lazy_static::lazy_static! {
    /// Base url for server generated urls. This is the base reference for eg. password invitation links.
    /// 
    /// Field name: `BASE_URL`
    pub static ref BASE_URL: String = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_owned());
}

lazy_static::lazy_static! {
    /// Database connection string.
    /// 
    /// Field name: `DATABASE_URL`
    pub static ref DATABASE_URL: String = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
}

lazy_static::lazy_static! {
    /// Salt for password hashing.
    /// 
    /// Field name: `PASSWORD_SALT`
    pub static ref PASSWORD_SALT: String = std::env::var("PASSWORD_SALT").unwrap_or_else(|_| "0123".repeat(8));
}

lazy_static::lazy_static! {
    /// Encryption key for authentication cookies.
    /// 
    /// Field name: `COOKIE_ENCRYPTION_KEY`
    pub static ref COOKIE_ENCRYPTION_KEY: String = std::env::var("COOKIE_ENCRYPTION_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

lazy_static::lazy_static! {
    /// Server path to store uploaded images.
    /// 
    /// Field name: `IMAGE_PATH`
    pub static ref IMAGE_PATH: String = std::env::var("IMAGE_PATH").unwrap_or_else(|_| "img/".to_owned());
}

lazy_static::lazy_static! {
    /// Header secret to access cron urls.
    /// 
    /// Field name: `CRON_SECRET`
    pub static ref CRON_SECRET: String = std::env::var("CRON_SECRET").expect("CRON_SECRET must be set.");
}

lazy_static::lazy_static! {
    /// Source address for mails, eg:
    /// payments@ascii.coffee
    /// 
    /// Field name: `MAIL_SENDER`
    pub static ref MAIL_SENDER: String = std::env::var("MAIL_SENDER").expect("MAIL_SENDER must be set.");
}

lazy_static::lazy_static! {
    /// Sender name for mails, eg:
    /// "Ascii Pay Service"
    /// 
    /// Field name: `MAIL_SENDER_NAME`
    pub static ref MAIL_SENDER_NAME: String = std::env::var("MAIL_SENDER_NAME").expect("MAIL_SENDER_NAME must be set.");
}

lazy_static::lazy_static! {
    /// Mail server url. Server can be set to test mode if url ends with `.local` eg: `mail.example.local`.
    /// Otherwise the url is used to verify tls certificates.
    /// 
    /// Field name: `MAIL_URL`
    pub static ref MAIL_SERVER: String = std::env::var("MAIL_URL").expect("MAIL_URL must be set");
}

lazy_static::lazy_static! {
    /// Login username for mail server.
    /// 
    /// Field name: `MAIL_USER`
    pub static ref MAIL_USER: String = std::env::var("MAIL_USER").expect("MAIL_USER must be set");
}

lazy_static::lazy_static! {
    /// Login password for mail server.
    /// 
    /// Field name: `MAIL_PASSWORD`
    pub static ref MAIL_PASS: String = std::env::var("MAIL_PASSWORD").expect("MAIL_PASSWORD must be set");
}
