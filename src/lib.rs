#[cfg(feature = "client")]
use client_imports::*;

use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
mod client_imports {
    pub use crate::error::JsonRpcError;
    pub use crate::params::Params;
    pub use anyhow::Error;
    pub use reqwest::{Client as ClientR, Url};
    pub use serde::de::DeserializeOwned;
    pub use serde::Deserialize;
    pub use serde_json::{from_value, json, Value};
    pub use std::sync::atomic::{AtomicU64, Ordering};
    pub use std::sync::Arc;
    pub use std::time::Duration;
}

pub mod error;
pub mod params;

#[cfg(feature = "client")]
#[derive(Clone, Debug)]
pub struct Client {
    inner: ClientR,
    url: Url,
    id: Arc<AtomicU64>,
}

#[cfg(feature = "client")]
impl Client {
    #[must_use]
    pub fn new(url: &Url, timeout: Option<Duration>, connection_timeout: Option<Duration>) -> Self {
        let builder = ClientR::builder();
        let client = builder
            .connect_timeout(connection_timeout.unwrap_or_else(|| Duration::from_secs(10)))
            .timeout(timeout.unwrap_or_else(|| Duration::from_secs(5)))
            .build()
            .expect("Shouldn't happen on this set of options");
        //todo check correctness

        Client {
            inner: client,
            url: url.clone(),
            id: Arc::new(AtomicU64::new(0)),
        }
    }

    ///Creates request
    /// ```
    /// {
    ///     "jsonrpc": "2.0",
    ///     "method": M,
    ///     "id": id,
    ///     "params": P
    /// }
    /// ```
    pub async fn request<M, P, Ret>(&self, method: M, params: P) -> Result<Ret, Error>
    where
        M: AsRef<str> + Send,
        P: Serialize,
        Ret: DeserializeOwned,
    {
        #[derive(Deserialize, Debug)]
        struct JsonRpcData {
            result: Option<Value>,
            error: Option<Value>,
        }

        let client = &self.inner;
        let id = { self.id.fetch_add(1, Ordering::SeqCst) };

        let json_payload = json!({
            "jsonrpc": 2.0,
            "method": method.as_ref(),
            "params": params,
            "id": id
        });

        let data: JsonRpcData = client
            .post(self.url.clone())
            .json(&json_payload)
            .send()
            .await?
            .json()
            .await?;
        match data.error {
            Some(a) => Err(parse_error(a)?.into()),
            None => match data.result {
                Some(a) => Ok(from_value(a)?),
                None => Err(Error::msg("Bad server  answer")),
            },
        }
    }
}

#[cfg(feature = "client")]
fn parse_error(value: Value) -> Result<JsonRpcError, Error> {
    #[derive(Deserialize)]
    struct ErrorObj {
        code: i32,
        message: String,
        data: Option<String>,
    }
    let error_obj: ErrorObj = serde_json::from_value(value)?;

    Ok(JsonRpcError {
        code: error_obj.code,
        data: error_obj.data,
        message: error_obj.message,
    })
}
