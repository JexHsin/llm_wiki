use std::fs;
use std::io::Read;
use std::path::Path;
use std::thread;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Response, Server, StatusCode};

const DEFAULT_PUBLIC_PORT: u16 = 19830;
const UPSTREAM: &str = "http://127.0.0.1:19828";
const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn start_web_api_proxy(app: AppHandle) {
    thread::spawn(move || {
        let port = std::env::var("LLM_WIKI_WEB_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PUBLIC_PORT);
        let bind = format!("0.0.0.0:{port}");
        let server = match Server::http(&bind) {
            Ok(server) => server,
            Err(err) => {
                eprintln!("[Web API Proxy] failed to bind {bind}: {err}");
                return;
            }
        };
        eprintln!("[Web API Proxy] Listening on http://{bind} and forwarding to {UPSTREAM}");

        let client = reqwest::Client::new();
        for request in server.incoming_requests() {
            let client = client.clone();
            let app = app.clone();
            thread::spawn(move || handle_request(app, client, request));
        }
    });
}

fn handle_request(app: AppHandle, client: reqwest::Client, mut request: tiny_http::Request) {
    if request.method() == &Method::Options {
        respond_options(request);
        return;
    }

    if request.method() == &Method::Get {
        if let Some((project_id, query)) = lint_route(request.url()) {
            handle_lint(app, request, &project_id, &query);
            return;
        }
    }

    let url = format!("{UPSTREAM}{}", request.url());
    let method = match request.method() {
        &Method::Get => reqwest::Method::GET,
        &Method::Post => reqwest::Method::POST,
        _ => {
            respond_value(request, 405, json!({ "ok": false, "error": "Method not allowed" }));
            return;
        }
    };

    let body_result = {
        let mut limited = request.as_reader().take(MAX_BODY_BYTES as u64 + 1);
        let mut body = Vec::new();
        limited.read_to_end(&mut body).map(|_| body)
    };
    let body = match body_result {
        Ok(body) => body,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": format!("failed to read request body: {err}") }));
            return;
        }
    };
    if body.len() > MAX_BODY_BYTES {
        respond_value(request, 413, json!({ "ok": false, "error": "Request body too large" }));
        return;
    }

    let mut builder = client.request(method, url);
    for header in request.headers() {
        let key = header.field.as_str();
        if matches!(key.to_ascii_lowercase().as_str(), "authorization" | "content-type" | "x-llm-wiki-token") {
            builder = builder.header(key, header.value.as_str());
        }
    }
    if !body.is_empty() {
        builder = builder.body(body);
    }

    let result = tauri::async_runtime::block_on(async move {
        match builder.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let bytes = resp.bytes().await.map(|b| b.to_vec()).unwrap_or_default();
                (status, bytes)
            }
            Err(err) => {
                let body = json!({ "ok": false, "error": format!("upstream API request failed: {err}") });
                (502, body.to_string().into_bytes())
            }
        }
    });

    respond_json(request, result.0, result.1);
}

fn lint_route(url: &str) -> Option<(String, String)> {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let parts = path
        .trim_start_matches("/api/v1/")
        .split('/')
        .collect::<Vec<_>>();
    match parts.as_slice() {
        ["projects", project_id, "lint"] => Some((percent_decode(project_id), query.to_string())),
        _ => None,
    }
}

fn handle_lint(app: AppHandle, request: tiny_http::Request, project_id: &str, query: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };

    let limit = query_param(query, "limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1000)
        .clamp(1, 1000);
    let item_type = query_param(query, "type");
    let path = Path::new(&project_path).join(".llm-wiki/lint.json");
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => "[]".to_string(),
        Err(err) => {
            respond_value(request, 500, json!({ "ok": false, "error": format!("Failed to read lint state: {err}") }));
            return;
        }
    };
    let parsed = match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Array(items)) => items,
        Ok(_) => {
            respond_value(request, 500, json!({ "ok": false, "error": "Invalid lint state JSON: expected an array" }));
            return;
        }
        Err(err) => {
            respond_value(request, 500, json!({ "ok": false, "error": format!("Invalid lint state JSON: {err}") }));
            return;
        }
    };

    let items = parsed
        .into_iter()
        .filter(|item| {
            item_type
                .as_deref()
                .map(|expected| item.get("type").and_then(Value::as_str) == Some(expected))
                .unwrap_or(true)
        })
        .take(limit)
        .collect::<Vec<_>>();

    respond_value(request, 200, json!({
        "ok": true,
        "projectId": resolved_id,
        "count": items.len(),
        "lint": items,
    }));
}

fn resolve_project(app: &AppHandle, project_id: &str) -> Option<(String, String)> {
    let state = load_app_state(app)?;
    if project_id.eq_ignore_ascii_case("current") {
        if let Some(last) = state.get("lastProject") {
            if let Some(path) = last.get("path").and_then(Value::as_str) {
                let id = last
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or(project_id)
                    .to_string();
                return Some((id, path.to_string()));
            }
        }
    }
    if let Some(registry) = state.get("projectRegistry").and_then(Value::as_object) {
        for (id, value) in registry {
            let Some(path) = value.get("path").and_then(Value::as_str) else {
                continue;
            };
            if id == project_id || normalize_path(path) == normalize_path(project_id) {
                return Some((id.clone(), path.to_string()));
            }
        }
    }
    if let Some(recents) = state.get("recentProjects").and_then(Value::as_array) {
        for value in recents {
            let Some(path) = value.get("path").and_then(Value::as_str) else {
                continue;
            };
            let id = value.get("id").and_then(Value::as_str).unwrap_or(project_id);
            if id == project_id || normalize_path(path) == normalize_path(project_id) {
                return Some((id.to_string(), path.to_string()));
            }
        }
    }
    None
}

fn load_app_state(app: &AppHandle) -> Option<Value> {
    let path = app.path().app_data_dir().ok()?.join("app-state.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if percent_decode(k) == key {
            return Some(percent_decode(v));
        }
    }
    None
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_string()
}

fn respond_options(request: tiny_http::Request) {
    let mut response = Response::empty(StatusCode(204));
    add_cors(&mut response);
    response.add_header(Header::from_bytes("Access-Control-Max-Age", "600").unwrap());
    let _ = request.respond(response);
}

fn respond_value(request: tiny_http::Request, status: u16, body: Value) {
    respond_json(request, status, body.to_string().into_bytes());
}

fn respond_json(request: tiny_http::Request, status: u16, body: Vec<u8>) {
    let mut response = Response::from_data(body).with_status_code(StatusCode(status));
    add_cors(&mut response);
    response.add_header(Header::from_bytes("Content-Type", "application/json").unwrap());
    let _ = request.respond(response);
}

fn add_cors<R: std::io::Read>(response: &mut Response<R>) {
    response.add_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap());
    response.add_header(Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap());
    response.add_header(Header::from_bytes("Access-Control-Allow-Headers", "Content-Type, Authorization, X-LLM-Wiki-Token").unwrap());
}
