use std::io::Read;

use serde_json::{json, Value};
use tiny_http::{Header, Response, StatusCode};

const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn handle_web_chat(client: reqwest::Client, mut request: tiny_http::Request, project_id: String) {
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(llm_config) = body.get("llmConfig") else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing field: llmConfig" }));
        return;
    };
    let Some(messages) = body.get("messages").and_then(Value::as_array) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing array field: messages" }));
        return;
    };
    let provider = llm_config.get("provider").and_then(Value::as_str).unwrap_or("custom");
    let model = llm_config.get("model").and_then(Value::as_str).unwrap_or("");
    if model.trim().is_empty() {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing llmConfig.model" }));
        return;
    }
    let endpoint = match chat_completion_endpoint(llm_config) {
        Ok(endpoint) => endpoint,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": err }));
            return;
        }
    };

    let mut payload = json!({
        "model": model,
        "messages": messages,
        "stream": false,
    });
    if let Some(overrides) = body.get("requestOverrides").and_then(Value::as_object) {
        for (key, value) in overrides {
            payload[key] = value.clone();
        }
    }
    let api_key = llm_config.get("apiKey").and_then(Value::as_str).unwrap_or("").to_string();

    let result = tauri::async_runtime::block_on(async move {
        let mut builder = client
            .post(endpoint)
            .header("content-type", "application/json")
            .json(&payload);
        if !api_key.trim().is_empty() {
            builder = builder.bearer_auth(api_key);
        }
        match builder.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let text = resp.text().await.unwrap_or_default();
                if !(200..300).contains(&status) {
                    return (status, json!({ "ok": false, "error": text }).to_string().into_bytes());
                }
                match serde_json::from_str::<Value>(&text) {
                    Ok(json_body) => {
                        let answer = extract_chat_answer(&json_body);
                        (200, json!({
                            "ok": true,
                            "projectId": project_id,
                            "provider": provider,
                            "answer": answer,
                            "raw": json_body,
                        }).to_string().into_bytes())
                    }
                    Err(err) => (502, json!({ "ok": false, "error": format!("Invalid LLM JSON response: {err}") }).to_string().into_bytes()),
                }
            }
            Err(err) => (502, json!({ "ok": false, "error": format!("LLM request failed: {err}") }).to_string().into_bytes()),
        }
    });

    respond_json(request, result.0, result.1);
}

fn chat_completion_endpoint(config: &Value) -> Result<String, String> {
    let provider = config.get("provider").and_then(Value::as_str).unwrap_or("custom");
    match provider {
        "openai" => Ok("https://api.openai.com/v1/chat/completions".to_string()),
        "custom" => {
            let endpoint = config.get("customEndpoint").and_then(Value::as_str).unwrap_or("").trim();
            if endpoint.is_empty() {
                Err("Missing llmConfig.customEndpoint".to_string())
            } else if endpoint.ends_with("/chat/completions") {
                Ok(endpoint.to_string())
            } else {
                Ok(format!("{}/chat/completions", endpoint.trim_end_matches('/')))
            }
        }
        "ollama" => {
            let endpoint = config.get("ollamaUrl").and_then(Value::as_str).unwrap_or("").trim();
            if endpoint.is_empty() {
                Err("Missing llmConfig.ollamaUrl".to_string())
            } else {
                Ok(format!("{}/v1/chat/completions", endpoint.trim_end_matches('/')))
            }
        }
        other => Err(format!("Web Chat API currently supports openai/custom/ollama providers, got: {other}")),
    }
}

fn extract_chat_answer(json_body: &Value) -> String {
    let content = json_body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"));
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(value) => value.to_string(),
        None => "".to_string(),
    }
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
    respond_json(request, status, body.to_string().into_bytes());
}

fn respond_json(request: tiny_http::Request, status: u16, body: Vec<u8>) {
    let mut response = Response::from_data(body).with_status_code(StatusCode(status));
    response.add_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap());
    response.add_header(Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap());
    response.add_header(Header::from_bytes("Access-Control-Allow-Headers", "Content-Type, Authorization, X-LLM-Wiki-Token").unwrap());
    response.add_header(Header::from_bytes("Content-Type", "application/json").unwrap());
    let _ = request.respond(response);
}
