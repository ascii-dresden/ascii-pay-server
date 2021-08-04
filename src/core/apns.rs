use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::Identity;

use crate::core::env;

use super::ServiceResult;

pub fn send(push_token: &str) -> ServiceResult<u16> {
    let pkcs12_file = File::open(Path::new(env::APPLE_WALLET_PASS_CERTIFICATE.as_str()))?;
    let mut pkcs12_reader = BufReader::new(pkcs12_file);
    let mut pkcs12_buffer = Vec::new();
    pkcs12_reader.read_to_end(&mut pkcs12_buffer)?;

    let client = reqwest::blocking::Client::builder()
        .identity(Identity::from_pkcs12_der(
            &pkcs12_buffer,
            env::APPLE_WALLET_PASS_CERTIFICATE_PASSWORD.as_str(),
        )?)
        .http2_prior_knowledge()
        .build()?;

    let path = format!("https://api.push.apple.com:443/3/device/{}", push_token);
    let mut builder = client.post(&path).header(CONTENT_TYPE, "application/json");

    let payload_json = "{}";
    builder = builder.header(CONTENT_LENGTH, payload_json.len() as u64);

    let response = match builder.body(payload_json).send() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(e.into());
        }
    };

    Ok(response.status().as_u16())
}
