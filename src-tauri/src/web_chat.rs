use std::io::Read;

use serde_json::{json, Value};
use tiny_http::{Header, Response, StatusCode};

const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

pub fn handle_web_chat(client: reqwest::Client, mut request: tiny_http::Request, _project_id: String) {
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(url) = body.get("url").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: url" }));
        return;
    };
    if !is_allowed_http_url(url) {
        respond_value(request, 400, json!({ "ok": false, "error": "Chat provider URL must be http or https" }));
        return;
    }
    let Some(provider_body) = body.get("body") else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing field: body" }));
        return;
    };

    let mut builder = client.post(url);
    if let Some(headers) = body.get("headers").and_then(Value::as_object) {
        for (key, value) in headers {
            let Some(value) = value.as_str() else { continue };
            if is_safe_forward_header(key) {
                builder = builder.header(key, value);
            }
        }
    }
    builder = builder.body(provider_body.to_string());

    let result = tauri::async_runtime::block_on(async move {
        match builder.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let bytes = resp.bytes().await.map(|b| b.to_vec()).unwrap_or_default();
                (status, content_type, bytes)
            }
            Err(err) => {
                let body = json!({ "ok": false, "error": format!("LLM request failed: {err}") });
                (502, "application/json".to_string(), body.to_string().into_bytes())
            }
        }
    });

    respond_bytes(request, result.0, &result.1, result.2);
}

fn is_allowed_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn is_safe_forward_header(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    !matches!(
        lower.as_str(),
        "host" | "content-length" | "connection" | "transfer-encoding" | "accept-encoding"
    )
}

fn read_body(request: &mut tiny_http::Request) -> Result<Vec<u8>, (u16, Value)> {
    let mut limited = request.as_reader().take(MAX_BODY_BYTES as u64 + 1);
    let mut body = Vec::new();
    if let Err(err) = limited.read_to_end(&mut body) {
        return Err((400, json!({ "ok": false, "error": format!("failed to read request body: {err}") })));
    }
    if body.len() > MAX_BODY_BYTES {
        return Err((413, json!({ "ok": false, "error": "Request body too large" })));
    }
    Ok(body)
}

fn read_json_body(request: &mut tiny_http::Request) -> Result<Value, (u16, Value)> {
    let body = read_body(request)?;
    if body.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice::<Value>(&body)
        .map_err(|err| (400, json!({ "ok": false, "error": format!("Invalid JSON body: {err}") })))
}

fn respond_value(request: tiny_http::Request, status: u16, body: Value) {
    respond_bytes(request, status, "application/json", body.to_string().into_bytes());
}

fn respond_bytes(request: tiny_http::Request, status: u16, content_type: &str, body: Vec<u8>) {
    let mut response = Response::from_data(body).with_status_code(StatusCode(status));
    response.add_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap());
    response.add_header(Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap());
    response.add_header(Header::from_bytes("Access-Control-Allow-Headers", "Content-Type, Authorization, X-LLM-Wiki-Token").unwrap());
    response.add_header(Header::from_bytes("Content-Type", content_type).unwrap());
    let _ = request.respond(response);
}
