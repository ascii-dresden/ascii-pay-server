use std::sync::Arc;

use aide::{
    axum::{routing::get, ApiRouter, IntoApiResponse},
    openapi::{OpenApi, Tag},
    redoc::Redoc,
    transform::TransformOpenApi,
};
use axum::{response::IntoResponse, Extension, Json};

pub fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("ascii-pay-server")
        .summary("A prepaid system")
        .description(include_str!("../README.md"))
        .tag(Tag {
            name: "accounts".into(),
            description: Some("Account management".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "account_authentication".into(),
            description: Some("Account authentication methods".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "products".into(),
            description: Some("Product management".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "product_image".into(),
            description: Some("Product images".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "auth".into(),
            description: Some("Session management".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "transactions".into(),
            description: Some("Transaction management".into()),
            ..Default::default()
        })
        .security_scheme(
            "SessionToken",
            aide::openapi::SecurityScheme::Http {
                scheme: "bearer".into(),
                bearer_format: None,
                description: Some("A key that is ignored.".into()),
                extensions: Default::default(),
            },
        )
}

pub fn docs_routes() -> ApiRouter {
    // We infer the return types for these routes
    // as an example.
    //
    // As a result, the `serve_redoc` route will
    // have the `text/html` content-type correctly set
    // with a 200 status.
    aide::gen::infer_responses(true);

    let router = ApiRouter::new()
        .route(
            "/",
            get(Redoc::new("/docs/api.json")
                .with_title("ascii-pay")
                .axum_handler()),
        )
        .route("/api.json", get(serve_docs));

    // Afterwards we disable response inference because
    // it might be incorrect for other routes.
    aide::gen::infer_responses(false);

    router
}

async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api).into_response()
}
