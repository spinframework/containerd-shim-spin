use std::str::from_utf8;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use spin_sdk::{redis, redis_subscriber, variables};

#[redis_subscriber]
async fn on_message(message: Bytes) -> Result<()> {
    let address = variables::get("redis_address").await?;
    let conn = redis::Connection::open(&address).await?;

    println!("{}", from_utf8(&message)?);

    conn.set(
        "redis-trigger-app-key",
        &"redis-trigger-app-value".to_owned().into_bytes(),
    )
    .await
    .map_err(|_| anyhow!("Error executing Redis set command"))?;

    Ok(())
}
