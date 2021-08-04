use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use actix_http::client::Connector;
use openssl::{
    pkcs12::Pkcs12,
    ssl::{SslConnector, SslMethod},
};

use crate::core::env;

use super::ServiceResult;

pub async fn send(push_token: &str) -> ServiceResult<u16> {
    let pkcs12_file = File::open(Path::new(env::APPLE_WALLET_PASS_CERTIFICATE.as_str()))?;
    let mut pkcs12_reader = BufReader::new(pkcs12_file);
    let mut pkcs12_buffer = Vec::new();
    pkcs12_reader.read_to_end(&mut pkcs12_buffer)?;

    let pkcs = Pkcs12::from_der(&pkcs12_buffer)?
        .parse(&env::APPLE_WALLET_PASS_CERTIFICATE_PASSWORD.as_str())?;

    let mut connector_builder = SslConnector::builder(SslMethod::tls())?;
    connector_builder.set_certificate(&pkcs.cert)?;
    connector_builder.set_private_key(&pkcs.pkey)?;
    connector_builder.set_alpn_protos(b"\x02h2")?;
    let ssl_connector = connector_builder.build();

    let connector = Connector::new().ssl(ssl_connector).finish();

    let client = actix_web::client::Client::builder()
        .connector(connector)
        .finish();

    let path = format!("https://api.push.apple.com:443/3/device/{}", push_token);
    let mut builder = client
        .post(&path)
        .version(http::Version::HTTP_2)
        .content_type("application/json");

    let payload_json = "{}";
    builder = builder.content_length(payload_json.len() as u64);

    let response = match builder.send_body(payload_json).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(e.into());
        }
    };

    Ok(response.status().as_u16())
}
