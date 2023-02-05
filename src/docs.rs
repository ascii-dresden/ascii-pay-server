use std::sync::Arc;

use aide::{
    axum::{
        routing::{get, get_with},
        ApiRouter, IntoApiResponse,
    },
    openapi::{OpenApi, Tag},
    redoc::Redoc,
    transform::TransformOpenApi,
};
use axum::{response::IntoResponse, Extension, Json};

pub fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("Aide axum Open API")
        .summary("An example Todo application")
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
            "ApiKey",
            aide::openapi::SecurityScheme::ApiKey {
                location: aide::openapi::ApiKeyLocation::Header,
                name: "X-Auth-Key".into(),
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
        .api_route_with(
            "/",
            get_with(
                Redoc::new("/docs/api.json")
                    .with_title("ascii-pay")
                    .axum_handler(),
                |op| op.description("This documentation page."),
            ),
            |p| p.security_requirement("ApiKey"),
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
