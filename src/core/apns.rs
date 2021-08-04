use std::io::Read;

use a2::{request::payload::Payload, Endpoint, Response};
use actix_http::http::StatusCode;
use actix_web::client::Connector;
use openssl::{
    pkcs12::Pkcs12,
    ssl::{SslConnector, SslMethod},
};

use crate::core::ServiceError;

use super::ServiceResult;

pub async fn send<R>(
    certificate: &mut R,
    password: &str,
    endpoint: Endpoint,
    payload: Payload<'_>,
) -> ServiceResult<Response>
where
    R: Read,
{
    let mut cert_der: Vec<u8> = Vec::new();
    certificate.read_to_end(&mut cert_der)?;

    let pkcs = Pkcs12::from_der(&cert_der)?.parse(password)?;

    let mut connector_builder = SslConnector::builder(SslMethod::tls())?;
    connector_builder.set_certificate(&pkcs.cert)?;
    connector_builder.set_private_key(&pkcs.pkey)?;
    let ssl_connector = connector_builder.build();

    let connector = Connector::new().ssl(ssl_connector).finish();

    let client = actix_web::client::Client::builder()
        .connector(connector)
        .finish();

    let path = format!("https://{}/3/device/{}", endpoint, payload.device_token);
    let mut builder = client.post(&path).content_type("application/json");

    if let Some(ref apns_priority) = payload.options.apns_priority {
        builder = builder.header("apns-priority", format!("{}", apns_priority).as_bytes());
    }
    if let Some(ref apns_id) = payload.options.apns_id {
        builder = builder.header("apns-id", apns_id.as_bytes());
    }
    if let Some(ref apns_expiration) = payload.options.apns_expiration {
        builder = builder.header("apns-expiration", format!("{}", apns_expiration).as_bytes());
    }
    if let Some(ref apns_collapse_id) = payload.options.apns_collapse_id {
        builder = builder.header(
            "apns-collapse-id",
            apns_collapse_id.value.to_string().as_bytes(),
        );
    }
    if let Some(ref apns_topic) = payload.options.apns_topic {
        builder = builder.header("apns-topic", apns_topic.as_bytes());
    }

    let payload_json = payload.to_json_string().unwrap();
    builder = builder.content_length(payload_json.len() as u64);
    let mut response = builder.send_body(payload_json).await?;

    let apns_id = response
        .headers()
        .get("apns-id")
        .and_then(|s| s.to_str().ok())
        .map(|id| id.to_owned());

    match response.status() {
        StatusCode::OK => Ok(Response {
            apns_id,
            error: None,
            code: response.status().as_u16(),
        }),
        status => {
            let body = response.body().await?;

            Err(ServiceError::InternalServerError(
                "Error parsing APNS response!",
                format!(
                    "{:?}",
                    Response {
                        apns_id,
                        error: serde_json::from_slice(&body).ok(),
                        code: status.as_u16(),
                    }
                ),
            ))
        }
    }
}
