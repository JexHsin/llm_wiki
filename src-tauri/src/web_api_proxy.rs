use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::thread;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::commands;

#[path = "web_chat.rs"]
mod web_chat;

const DEFAULT_PUBLIC_PORT: u16 = 19830;
const UPSTREAM: &str = "http://127.0.0.1:19828";
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

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
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_lint(app, request, &project_id, &query);
            return;
        }
    }

    if request.method() == &Method::Post {
        if create_project_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_create_project(request);
            return;
        }
        if let Some(project_id) = chat_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            web_chat::handle_web_chat(client, request, project_id);
            return;
        }
        if let Some(project_id) = read_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_project_read(app, request, &project_id);
            return;
        }
        if let Some(project_id) = read_base64_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_project_read_base64(app, request, &project_id);
            return;
        }
        if let Some(project_id) = metadata_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_project_metadata(app, request, &project_id);
            return;
        }
        if let Some(project_id) = preprocess_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_project_preprocess(app, request, &project_id);
            return;
        }
        if let Some(project_id) = related_pages_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_related_pages(app, request, &project_id);
            return;
        }
        if let Some((project_id, action)) = vector_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_project_vector(app, request, &project_id, &action);
            return;
        }
        if let Some((project_id, action)) = copy_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_project_copy(app, request, &project_id, &action);
            return;
        }
        if let Some((project_id, action)) = write_route(request.url()) {
            if let Some((status, body)) = ensure_api_access(&app, request.url(), request.headers()) {
                respond_value(request, status, body);
                return;
            }
            handle_project_write(app, request, &project_id, &action);
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

    let body = match read_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };

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

fn route_parts(url: &str) -> Vec<&str> {
    let (path, _) = url.split_once('?').unwrap_or((url, ""));
    path.trim_start_matches("/api/v1/")
        .split('/')
        .collect::<Vec<_>>()
}

fn lint_route(url: &str) -> Option<(String, String)> {
    let (_, query) = url.split_once('?').unwrap_or((url, ""));
    match route_parts(url).as_slice() {
        ["projects", project_id, "lint"] => Some((percent_decode(project_id), query.to_string())),
        _ => None,
    }
}

fn create_project_route(url: &str) -> bool {
    let (path, _) = url.split_once('?').unwrap_or((url, ""));
    path == "/api/v1/projects/create"
}

fn chat_route(url: &str) -> Option<String> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "chat"] => Some(percent_decode(project_id)),
        _ => None,
    }
}

fn read_route(url: &str) -> Option<String> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "files", "read"] => Some(percent_decode(project_id)),
        _ => None,
    }
}

fn read_base64_route(url: &str) -> Option<String> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "files", "read-base64"] => Some(percent_decode(project_id)),
        _ => None,
    }
}

fn metadata_route(url: &str) -> Option<String> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "files", "metadata"] => Some(percent_decode(project_id)),
        _ => None,
    }
}

fn preprocess_route(url: &str) -> Option<String> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "files", "preprocess"] => Some(percent_decode(project_id)),
        _ => None,
    }
}

fn related_pages_route(url: &str) -> Option<String> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "wiki", "related-pages"] => Some(percent_decode(project_id)),
        _ => None,
    }
}

fn vector_route(url: &str) -> Option<(String, String)> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "vectors", "chunks", "upsert"] => Some((percent_decode(project_id), "chunks_upsert".to_string())),
        ["projects", project_id, "vectors", "chunks", "search"] => Some((percent_decode(project_id), "chunks_search".to_string())),
        ["projects", project_id, "vectors", "pages", "delete"] => Some((percent_decode(project_id), "page_delete".to_string())),
        ["projects", project_id, "vectors", "chunks", "count"] => Some((percent_decode(project_id), "chunks_count".to_string())),
        ["projects", project_id, "vectors", "chunks", "clear"] => Some((percent_decode(project_id), "chunks_clear".to_string())),
        ["projects", project_id, "vectors", "chunks", "optimize"] => Some((percent_decode(project_id), "chunks_optimize".to_string())),
        ["projects", project_id, "vectors", "legacy", "count"] => Some((percent_decode(project_id), "legacy_count".to_string())),
        ["projects", project_id, "vectors", "legacy", "drop"] => Some((percent_decode(project_id), "legacy_drop".to_string())),
        _ => None,
    }
}

fn copy_route(url: &str) -> Option<(String, String)> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "files", "copy"] => Some((percent_decode(project_id), "copy_file".to_string())),
        ["projects", project_id, "directories", "copy"] => Some((percent_decode(project_id), "copy_directory".to_string())),
        _ => None,
    }
}

fn write_route(url: &str) -> Option<(String, String)> {
    match route_parts(url).as_slice() {
        ["projects", project_id, "files", "write"] => Some((percent_decode(project_id), "write_file".to_string())),
        ["projects", project_id, "files", "write-base64"] => Some((percent_decode(project_id), "write_file_base64".to_string())),
        ["projects", project_id, "files", "write-atomic"] => Some((percent_decode(project_id), "write_file_atomic".to_string())),
        ["projects", project_id, "files", "delete"] => Some((percent_decode(project_id), "delete_file".to_string())),
        ["projects", project_id, "directories", "create"] => Some((percent_decode(project_id), "create_directory".to_string())),
        _ => None,
    }
}

fn handle_create_project(mut request: tiny_http::Request) {
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(name) = body.get("name").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: name" }));
        return;
    };
    let Some(path) = body.get("path").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: path" }));
        return;
    };
    match commands::project::create_project(name.to_string(), path.to_string()) {
        Ok(project) => respond_value(request, 200, json!({
            "ok": true,
            "name": project.name,
            "path": project.path,
        })),
        Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
    }
}

fn handle_project_read(app: AppHandle, mut request: tiny_http::Request, project_id: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(raw_path) = body.get("path").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: path" }));
        return;
    };
    let extract_images = body.get("extractImages").and_then(Value::as_bool);
    let target = match resolve_project_scoped_path(&project_path, raw_path) {
        Ok(path) => path,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let target_str = normalize_path(&target.to_string_lossy());
    match tauri::async_runtime::block_on(commands::fs::read_file(target_str.clone(), extract_images)) {
        Ok(content) => respond_value(request, 200, json!({
            "ok": true,
            "projectId": resolved_id,
            "path": target_str,
            "content": content,
        })),
        Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
    }
}

fn handle_project_read_base64(app: AppHandle, mut request: tiny_http::Request, project_id: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };
    let Some(raw_path) = body_string(&mut request, "path") else {
        return;
    };
    let target = match resolve_project_scoped_path(&project_path, &raw_path) {
        Ok(path) => path,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let target_str = normalize_path(&target.to_string_lossy());
    match tauri::async_runtime::block_on(commands::fs::read_file_as_base64(target_str.clone())) {
        Ok(file) => respond_value(request, 200, json!({
            "ok": true,
            "projectId": resolved_id,
            "path": target_str,
            "base64": file.base64,
            "mimeType": file.mime_type,
        })),
        Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
    }
}

fn body_string(request: &mut tiny_http::Request, field: &str) -> Option<String> {
    let body = match read_json_body(request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request.clone(), status, body);
            return None;
        }
    };
    body.get(field).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn handle_project_metadata(app: AppHandle, mut request: tiny_http::Request, project_id: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(raw_path) = body.get("path").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: path" }));
        return;
    };
    let target = match resolve_project_scoped_path(&project_path, raw_path) {
        Ok(path) => path,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let target_str = normalize_path(&target.to_string_lossy());
    let modified_time = match tauri::async_runtime::block_on(commands::fs::get_file_modified_time(target_str.clone())) {
        Ok(value) => value,
        Err(err) => {
            respond_value(request, 500, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let size = match tauri::async_runtime::block_on(commands::fs::get_file_size(target_str.clone())) {
        Ok(value) => value,
        Err(err) => {
            respond_value(request, 500, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let md5 = match tauri::async_runtime::block_on(commands::fs::get_file_md5(target_str.clone())) {
        Ok(value) => value,
        Err(err) => {
            respond_value(request, 500, json!({ "ok": false, "error": err }));
            return;
        }
    };
    respond_value(request, 200, json!({
        "ok": true,
        "projectId": resolved_id,
        "path": target_str,
        "modifiedTime": modified_time,
        "size": size,
        "md5": md5,
    }));
}

fn handle_project_preprocess(app: AppHandle, mut request: tiny_http::Request, project_id: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(raw_path) = body.get("path").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: path" }));
        return;
    };
    let target = match resolve_project_scoped_path(&project_path, raw_path) {
        Ok(path) => path,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let target_str = normalize_path(&target.to_string_lossy());
    match tauri::async_runtime::block_on(commands::fs::preprocess_file(target_str.clone())) {
        Ok(content) => respond_value(request, 200, json!({
            "ok": true,
            "projectId": resolved_id,
            "path": target_str,
            "content": content,
        })),
        Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
    }
}

fn handle_related_pages(app: AppHandle, mut request: tiny_http::Request, project_id: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(source_name) = body.get("sourceName").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: sourceName" }));
        return;
    };
    match tauri::async_runtime::block_on(commands::fs::find_related_wiki_pages(project_path, source_name.to_string())) {
        Ok(pages) => respond_value(request, 200, json!({
            "ok": true,
            "projectId": resolved_id,
            "pages": pages,
        })),
        Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
    }
}

fn handle_project_vector(app: AppHandle, mut request: tiny_http::Request, project_id: &str, action: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };

    match action {
        "chunks_upsert" => {
            let Some(page_id) = body.get("pageId").and_then(Value::as_str) else {
                respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: pageId" }));
                return;
            };
            let chunks_value = body.get("chunks").cloned().unwrap_or_else(|| json!([]));
            let chunks = match serde_json::from_value::<Vec<commands::vectorstore::ChunkUpsertInput>>(chunks_value) {
                Ok(chunks) => chunks,
                Err(err) => {
                    respond_value(request, 400, json!({ "ok": false, "error": format!("Invalid chunks payload: {err}") }));
                    return;
                }
            };
            match tauri::async_runtime::block_on(commands::vectorstore::vector_upsert_chunks(project_path, page_id.to_string(), chunks)) {
                Ok(()) => respond_value(request, 200, json!({ "ok": true, "projectId": resolved_id })),
                Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
            }
        }
        "chunks_search" => {
            let query_embedding = match serde_json::from_value::<Vec<f32>>(body.get("queryEmbedding").cloned().unwrap_or_else(|| json!([]))) {
                Ok(vec) => vec,
                Err(err) => {
                    respond_value(request, 400, json!({ "ok": false, "error": format!("Invalid queryEmbedding payload: {err}") }));
                    return;
                }
            };
            let top_k = body.get("topK").and_then(Value::as_u64).unwrap_or(10) as usize;
            match tauri::async_runtime::block_on(commands::vectorstore::vector_search_chunks(project_path, query_embedding, top_k)) {
                Ok(results) => respond_value(request, 200, json!({ "ok": true, "projectId": resolved_id, "results": results })),
                Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
            }
        }
        "page_delete" => {
            let Some(page_id) = body.get("pageId").and_then(Value::as_str) else {
                respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: pageId" }));
                return;
            };
            match tauri::async_runtime::block_on(commands::vectorstore::vector_delete_page(project_path, page_id.to_string())) {
                Ok(()) => respond_value(request, 200, json!({ "ok": true, "projectId": resolved_id })),
                Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
            }
        }
        "chunks_count" => match tauri::async_runtime::block_on(commands::vectorstore::vector_count_chunks(project_path)) {
            Ok(count) => respond_value(request, 200, json!({ "ok": true, "projectId": resolved_id, "count": count })),
            Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
        },
        "chunks_clear" => match tauri::async_runtime::block_on(commands::vectorstore::vector_clear_chunks(project_path)) {
            Ok(()) => respond_value(request, 200, json!({ "ok": true, "projectId": resolved_id })),
            Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
        },
        "chunks_optimize" => match tauri::async_runtime::block_on(commands::vectorstore::vector_optimize_chunks(project_path)) {
            Ok(()) => respond_value(request, 200, json!({ "ok": true, "projectId": resolved_id })),
            Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
        },
        "legacy_count" => match tauri::async_runtime::block_on(commands::vectorstore::vector_legacy_row_count(project_path)) {
            Ok(count) => respond_value(request, 200, json!({ "ok": true, "projectId": resolved_id, "count": count })),
            Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
        },
        "legacy_drop" => match tauri::async_runtime::block_on(commands::vectorstore::vector_drop_legacy(project_path)) {
            Ok(()) => respond_value(request, 200, json!({ "ok": true, "projectId": resolved_id })),
            Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
        },
        _ => respond_value(request, 400, json!({ "ok": false, "error": format!("Unsupported vector action: {action}") })),
    }
}

fn handle_project_copy(app: AppHandle, mut request: tiny_http::Request, project_id: &str, action: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(raw_source) = body.get("source").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: source" }));
        return;
    };
    let Some(raw_destination) = body.get("destination").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: destination" }));
        return;
    };
    let source = match resolve_project_scoped_path(&project_path, raw_source) {
        Ok(path) => path,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let destination = match resolve_project_scoped_path(&project_path, raw_destination) {
        Ok(path) => path,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let source = normalize_path(&source.to_string_lossy());
    let destination = normalize_path(&destination.to_string_lossy());
    match action {
        "copy_file" => match tauri::async_runtime::block_on(commands::fs::copy_file(source, destination.clone())) {
            Ok(()) => respond_value(request, 200, json!({
                "ok": true,
                "projectId": resolved_id,
                "path": destination,
            })),
            Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
        },
        "copy_directory" => match tauri::async_runtime::block_on(commands::fs::copy_directory(source, destination)) {
            Ok(files) => respond_value(request, 200, json!({
                "ok": true,
                "projectId": resolved_id,
                "files": files,
            })),
            Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
        },
        _ => respond_value(request, 400, json!({ "ok": false, "error": format!("Unsupported copy action: {action}") })),
    }
}

fn handle_project_write(app: AppHandle, mut request: tiny_http::Request, project_id: &str, action: &str) {
    let Some((resolved_id, project_path)) = resolve_project(&app, project_id) else {
        respond_value(request, 404, json!({ "ok": false, "error": format!("Unknown project: {project_id}") }));
        return;
    };
    let body = match read_json_body(&mut request) {
        Ok(body) => body,
        Err((status, body)) => {
            respond_value(request, status, body);
            return;
        }
    };
    let Some(raw_path) = body.get("path").and_then(Value::as_str) else {
        respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: path" }));
        return;
    };
    let target = match resolve_project_scoped_path(&project_path, raw_path) {
        Ok(path) => path,
        Err(err) => {
            respond_value(request, 400, json!({ "ok": false, "error": err }));
            return;
        }
    };
    let target_str = normalize_path(&target.to_string_lossy());

    let result = match action {
        "write_file" => {
            let Some(contents) = body.get("contents").and_then(Value::as_str) else {
                respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: contents" }));
                return;
            };
            tauri::async_runtime::block_on(commands::fs::write_file(target_str.clone(), contents.to_string()))
        }
        "write_file_base64" => {
            let Some(base64) = body.get("base64").and_then(Value::as_str) else {
                respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: base64" }));
                return;
            };
            tauri::async_runtime::block_on(commands::fs::write_file_base64(target_str.clone(), base64.to_string()))
        }
        "write_file_atomic" => {
            let Some(contents) = body.get("contents").and_then(Value::as_str) else {
                respond_value(request, 400, json!({ "ok": false, "error": "Missing string field: contents" }));
                return;
            };
            tauri::async_runtime::block_on(commands::fs::write_file_atomic(target_str.clone(), contents.to_string()))
        }
        "delete_file" => tauri::async_runtime::block_on(commands::fs::delete_file(target_str.clone())),
        "create_directory" => tauri::async_runtime::block_on(commands::fs::create_directory(target_str.clone())),
        _ => Err(format!("Unsupported write action: {action}")),
    };

    match result {
        Ok(()) => respond_value(request, 200, json!({
            "ok": true,
            "projectId": resolved_id,
            "path": target_str,
        })),
        Err(err) => respond_value(request, 500, json!({ "ok": false, "error": err })),
    }
}

fn resolve_project_scoped_path(project_path: &str, raw_path: &str) -> Result<PathBuf, String> {
    let project_root = PathBuf::from(project_path);
    let project_norm = normalize_path(project_path);
    let raw_norm = normalize_path(raw_path);
    let relative = if raw_norm == project_norm {
        ""
    } else if let Some(stripped) = raw_norm.strip_prefix(&format!("{project_norm}/")) {
        stripped
    } else {
        raw_norm.trim_start_matches('/')
    };

    let mut safe_relative = PathBuf::new();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => safe_relative.push(part),
            Component::CurDir => {}
            Component::ParentDir => return Err("Path escapes project root".to_string()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("Absolute path outside project root is not allowed".to_string())
            }
        }
    }

    let joined = project_root.join(&safe_relative);
    let root_canon = project_root
        .canonicalize()
        .map_err(|err| format!("Failed to resolve project path: {err}"))?;
    if joined.exists() {
        let joined_canon = joined
            .canonicalize()
            .map_err(|err| format!("Failed to resolve path: {err}"))?;
        if !joined_canon.starts_with(&root_canon) {
            return Err("Resolved path escapes project root".to_string());
        }
        return Ok(joined_canon);
    }

    let mut ancestor = joined.parent();
    while let Some(parent) = ancestor {
        if parent.exists() {
            let parent_canon = parent
                .canonicalize()
                .map_err(|err| format!("Failed to resolve parent path: {err}"))?;
            if !parent_canon.starts_with(&root_canon) {
                return Err("Resolved parent escapes project root".to_string());
            }
            break;
        }
        ancestor = parent.parent();
    }
    Ok(joined)
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

fn ensure_api_access(app: &AppHandle, url: &str, headers: &[Header]) -> Option<(u16, Value)> {
    if !api_enabled(app) {
        return Some((503, json!({ "ok": false, "error": "API server is disabled in Settings → API Server" })));
    }
    if !is_authorized(app, url, headers) {
        return Some((401, json!({ "ok": false, "error": "Unauthorized" })));
    }
    None
}

fn is_authorized(app: &AppHandle, url: &str, headers: &[Header]) -> bool {
    if !api_auth_required(app) {
        return true;
    }
    let Some(token) = api_token(app) else {
        return false;
    };
    let (_, query) = url.split_once('?').unwrap_or((url, ""));
    if query_param(query, "token")
        .map(|value| constant_time_eq(value.as_bytes(), token.as_bytes()))
        .unwrap_or(false)
    {
        return true;
    }
    headers.iter().any(|header| {
        let key = header.field.as_str().to_ascii_lowercase();
        let value = header.value.as_str();
        if key == "x-llm-wiki-token" {
            return constant_time_eq(value.as_bytes(), token.as_bytes());
        }
        if key == "authorization" {
            return value
                .strip_prefix("Bearer ")
                .map(|v| constant_time_eq(v.as_bytes(), token.as_bytes()))
                .unwrap_or(false);
        }
        false
    })
}

fn api_auth_required(app: &AppHandle) -> bool {
    !api_allow_unauthenticated(app)
}

fn api_allow_unauthenticated(app: &AppHandle) -> bool {
    load_app_state(app)
        .and_then(|parsed| parsed.get("apiConfig").and_then(|v| v.get("allowUnauthenticated")).and_then(Value::as_bool))
        .unwrap_or(false)
}

fn api_enabled(app: &AppHandle) -> bool {
    load_app_state(app)
        .and_then(|parsed| parsed.get("apiConfig").and_then(|v| v.get("enabled")).and_then(Value::as_bool))
        .unwrap_or(true)
}

fn api_token(app: &AppHandle) -> Option<String> {
    if let Ok(token) = std::env::var("LLM_WIKI_API_TOKEN") {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    load_app_state(app)
        .and_then(|parsed| parsed.get("apiConfig").and_then(|v| v.get("token")).and_then(Value::as_str).map(ToOwned::to_owned))
        .filter(|token| !token.is_empty())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for i in 0..max_len {
        let a = left.get(i).copied().unwrap_or(0);
        let b = right.get(i).copied().unwrap_or(0);
        diff |= (a ^ b) as usize;
    }
    diff == 0
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
