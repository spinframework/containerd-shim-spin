use anyhow::Result;
use spin_sdk::{
    http::{FullBody, IntoResponse, Request, Response},
    http_service,
    key_value::Store,
};

/// A simple Spin HTTP component.
#[http_service]
async fn handle_kv(_req: Request) -> Result<impl IntoResponse> {
    let store = Store::open("foo").await?;
    store.set("mykey", "wow".as_bytes()).await?;
    let value = store.get("mykey").await?;
    Ok(Response::builder()
        .status(200)
        .body(FullBody::<bytes::Bytes>::from(value.unwrap_or_default()))?)
}
