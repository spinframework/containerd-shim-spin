use anyhow::{anyhow, Result};
use spin_sdk::{
    http::{body::IncomingBodyExt, FullBody, IntoResponse, Request, Response},
    http_service, redis,
};

// Expect redis running on localhost or in the same pod
const ADDRESS: &str = "redis://localhost:6379";
const KEY: &str = "lastEvent";

#[http_service]
async fn handle_spin_dapr(req: Request) -> Result<impl IntoResponse> {
    println!("{:?} {:?}\n", req.method(), req.uri());
    let conn = redis::Connection::open(ADDRESS).await?;

    if req.uri().path().ends_with("kevents") {
        let value = req.into_body().bytes().await?;
        println!("Set: {:?}", value);
        conn.set(KEY, &value)
            .await
            .map_err(|_| anyhow!("Error executing Redis set command"))?;
    }

    let value = conn
        .get(KEY)
        .await
        .map_err(|_| anyhow!("Error executing Redis get command"))?;

    Ok(Response::builder()
        .status(200)
        .header("foo", "bar")
        .body(FullBody::<bytes::Bytes>::from(value.unwrap_or_default()))?)
}
