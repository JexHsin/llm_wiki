@echo off
set LLM_WIKI_HTTP_HOST=0.0.0.0
if "%LLM_WIKI_HTTP_PORT%"=="" set LLM_WIKI_HTTP_PORT=19828

npm ci
npm run mcp:build
npm run build

cd src-tauri
cargo run --bin llm-wiki
