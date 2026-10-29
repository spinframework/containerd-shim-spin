use spin_sdk::http::Request;
use spin_sdk::http_service;

/// A simple Spin HTTP component.
#[http_service]
async fn hello_world(_req: Request) -> &'static str {
    "Hello world from Spin!"
}
