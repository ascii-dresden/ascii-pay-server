use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use log::error;
use reqwest::{header, Certificate, Client, Identity, Version};

use crate::{env, error::ServiceResult};

pub struct ApplePushNotificationService {
    client: Client,
}

impl ApplePushNotificationService {
    /// Create new APNS client
    pub fn new() -> ServiceResult<Self> {
        let pkcs12_file = File::open(Path::new(env::APPLE_WALLET_PASS_CERTIFICATE.as_str()))?;
        let mut pkcs12_reader = BufReader::new(pkcs12_file);
        let mut pkcs12_buffer = Vec::new();
        pkcs12_reader.read_to_end(&mut pkcs12_buffer)?;

        let pkcs = Identity::from_pkcs12_der(
            &pkcs12_buffer,
            env::APPLE_WALLET_PASS_CERTIFICATE_PASSWORD.as_str(),
        )?;

        let apns = load_x509(env::APPLE_WALLET_APNS_CERTIFICATE.as_str())?;
        let wwdr = load_x509(env::APPLE_WALLET_WWDR_CERTIFICATE.as_str())?;

        let client = Client::builder()
            .identity(pkcs)
            .add_root_certificate(apns)
            .add_root_certificate(wwdr)
            .http2_prior_knowledge()
            .build()?;

        Ok(Self { client })
    }

    /// Send message over APNS client
    pub async fn send(&self, push_token: &str) -> ServiceResult<u16> {
        let path = format!("https://api.push.apple.com:443/3/device/{}", push_token);
        let builder = self
            .client
            .post(&path)
            .version(Version::HTTP_2)
            .header(header::CONTENT_TYPE, "application/json");

        let payload_json = "{}";
        let request = builder
            .header(header::CONTENT_LENGTH, payload_json.len())
            .body(payload_json)
            .build()?;

        let response = match self.client.execute(request).await {
            Ok(v) => v,
            Err(e) => {
                error!("{:?}", e);
                return Err(e.into());
            }
        };

        Ok(response.status().as_u16())
    }
}

fn load_x509(path: &str) -> ServiceResult<Certificate> {
    let file = File::open(Path::new(path))?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    Ok(Certificate::from_pem(&buffer)?)
}
