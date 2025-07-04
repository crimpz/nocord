use crate::Result;
use crate::ctx::Ctx;
use crate::web::error::{ClientError, Error};
use crate::web::rpc::RpcInfo;
use hyper::{Method, Uri};
use serde::Serialize;
use serde_json::{Value, json};
use serde_with::skip_serializing_none;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tracing::debug;
use uuid::Uuid;

#[skip_serializing_none]
#[derive(Serialize)]
struct RequestLogLine {
    uuid: String,      // uuid string formated
    timestamp: String, // should be iso8601

    // User and context attributes
    user_id: Option<i64>,

    // http request attributes
    http_path: String,
    http_method: String,

    // rpc info
    rpc_method: Option<String>,

    // Error attributes
    client_error_type: Option<String>,
    error_type: Option<String>,
    error_data: Option<Value>,
}

pub async fn log_request(
    uuid: Uuid,
    method: Method,
    uri: Uri,
    rpc_info: Option<&RpcInfo>,
    ctx: Option<Ctx>,
    web_error: Option<&Error>,
    client_error: Option<ClientError>,
) -> Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let error_type = web_error.map(|se| se.as_ref().to_string());
    let error_data = serde_json::to_value(web_error)
        .ok()
        .and_then(|mut v| v.get_mut("data").map(|v| v.take()));

    // Create the RequestLogLine
    let log_line = RequestLogLine {
        uuid: uuid.to_string(),
        timestamp: timestamp.to_string(),

        http_path: uri.to_string(),
        http_method: method.to_string(),

        rpc_method: rpc_info.map(|rpc| rpc.method.to_string()),

        user_id: ctx.map(|c| c.user_id()),

        client_error_type: client_error.map(|e| e.as_ref().to_string()),

        error_type,
        error_data,
    };

    debug!("log_request: \n{}", json!(log_line));

    // TODO: Send to cloud-watch type service
    Ok(())
}
