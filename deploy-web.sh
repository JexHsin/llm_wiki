#!/usr/bin/env bash
set -euo pipefail

export LLM_WIKI_WEB_PORT="${LLM_WIKI_WEB_PORT:-19830}"

npm ci
npm run mcp:build
npm run build

cd src-tauri
cargo run --bin llm-wiki
