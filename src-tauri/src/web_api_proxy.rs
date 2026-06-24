use std::io::Read;
use std::thread;

use tiny_http::{Header, Method, Response, Server, StatusCode};

const DEFAULT_PUBLIC_PORT: u16 = 19830;
const UPSTREAM: &str = "http://127.0.0.1:19828";
const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn start_web_api_proxy() {
    thread::spawn(|| {
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
            thread::spawn(move || handle_request(client, request));
        }
    });
}

fn handle_request(client: reqwest::Client, mut request: tiny_http::Request) {
    if request.method() == &Method::Options {
        respond_options(request);
        return;
    }

    let url = format!("{UPSTREAM}{}", request.url());
    let method = match request.method() {
        &Method::Get => reqwest::Method::GET,
        &Method::Post => reqwest::Method::POST,
        _ => {
            respond_json(request, 405, br#"{"ok":false,"error":"Method not allowed"}"#.to_vec());
            return;
        }
    };

    let mut limited = request.as_reader().take(MAX_BODY_BYTES as u64 + 1);
    let mut body = Vec::new();
    if let Err(err) = limited.read_to_end(&mut body) {
        let msg = format!(r#"{{"ok":false,"error":"failed to read request body: {err}"}}"#);
        respond_json(request, 400, msg.into_bytes());
        return;
    }
    if body.len() > MAX_BODY_BYTES {
        respond_json(request, 413, br#"{"ok":false,"error":"Request body too large"}"#.to_vec());
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
                let msg = format!(r#"{{"ok":false,"error":"upstream API request failed: {err}"}}"#);
                (502, msg.into_bytes())
            }
        }
    });

    respond_json(request, result.0, result.1);
}

fn respond_options(request: tiny_http::Request) {
    let mut response = Response::empty(StatusCode(204));
    add_cors(&mut response);
    response.add_header(Header::from_bytes("Access-Control-Max-Age", "600").unwrap());
    let _ = request.respond(response);
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
