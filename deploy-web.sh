#!/usr/bin/env bash
set -euo pipefail

export LLM_WIKI_HTTP_HOST="${LLM_WIKI_HTTP_HOST:-0.0.0.0}"
export LLM_WIKI_HTTP_PORT="${LLM_WIKI_HTTP_PORT:-19828}"

npm ci
npm run mcp:build
npm run build

cd src-tauri
cargo run --bin llm-wiki
