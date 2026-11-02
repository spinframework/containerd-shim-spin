use spin_sdk::http::{FullBody, IntoResponse, Request, Response};
use spin_sdk::http_service;

/// A simple Spin HTTP component.
#[http_service]
async fn handle_http_trigger(_req: Request) -> anyhow::Result<impl IntoResponse> {
    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(FullBody::<bytes::Bytes>::from(
            "Hello world from multi trigger Spin!",
        ))?)
}
