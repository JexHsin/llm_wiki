# Web Equivalent Refactor

This branch is for a strict Web-equivalent refactor of `nashsu/llm_wiki`/current `main` implementation.

## Non-negotiable constraints

1. Preserve the original llm_wiki business logic.
2. Preserve the original Wiki generation mechanism.
3. Preserve entity, concept, source summary, graph, Review, Lint, Deep Research, Chat, MCP, and vector search behavior.
4. Remove only the Tauri desktop shell and replace the call boundary with HTTP.
5. Do not introduce a new knowledge-base framework.
6. Do not add new business features.
7. Do not change the Wiki directory structure or persisted data formats.
8. Keep existing tests and add equivalence tests.

## Correct migration strategy

The correct migration is not to build a new RAG engine. The original project already contains the RAG/wiki/graph/review/lint/deep-research logic in the existing TypeScript and Rust modules. The Web version must expose those same capabilities through HTTP.

### Phase 1: API boundary

- Add a single frontend HTTP client: `src/lib/http-command-client.ts`.
- Replace direct Tauri `invoke(...)` calls with typed HTTP wrappers.
- Keep request/response payloads identical to the old command payloads where possible.
- Use `src/commands/http-api.ts` for endpoint-level wrappers.
- Use `src/commands/web-equivalent.ts`, `src/commands/web-fs.ts`, and `src/commands/web-projects.ts` as adapter façades that convert API payloads back to existing frontend domain types.

### Completed read-only substitutions

When `VITE_LLM_WIKI_WEB_MODE=true`, the following read-only paths now use HTTP instead of direct Tauri invoke calls:

- `src/commands/fs.ts`
  - `readFile`
  - `listDirectory`
  - `openProject`
  - `apiServerStatus`
- `src/lib/persist.ts`
  - `loadReviewItems` uses the existing `/reviews` API instead of reading `.llm-wiki/review.json` directly.
- `src/commands/web-projects.ts`
  - project list/current project helpers are available for the project selection UI migration.

Desktop mode remains unchanged unless `VITE_LLM_WIKI_WEB_MODE=true` is set.

### Phase 2: Backend serviceization

- Move command implementations behind HTTP handlers.
- Reuse the existing Rust command modules.
- Do not duplicate file parsing, wiki generation, vector indexing, graph logic, review state, lint state, or chat logic.
- Preserve `.llm-wiki/`, `wiki/`, `raw/sources/`, `purpose.md`, `schema.md`, `index.md`, `log.md`, and `[[wikilink]]` semantics.

### Phase 3: Remove desktop-only integrations

Remove or replace only desktop shell integrations:

- window close/minimize behavior
- tray integration
- Tauri dialog plugin
- Tauri autostart plugin
- Tauri WebView bootstrap

Do not remove business modules used by ingest/query/lint/review/chat.

### Phase 4: Equivalence tests

Required test groups:

- project create/open layout equivalence
- source ingest equivalence
- wiki page generation equivalence
- index/log/schema/purpose preservation
- entity/concept extraction equivalence
- graph edge/node equivalence
- vector upsert/search/chunk search equivalence
- Review item persistence equivalence
- Lint output equivalence
- Deep Research result ingestion equivalence
- Chat response/context retrieval equivalence
- HTTP auth/CORS/0.0.0.0 bind tests
- Linux and Windows deploy smoke tests

## Transitional public API bridge

The original Rust API remains the source of truth on `127.0.0.1:19828`. To avoid rewriting or duplicating business logic, this branch adds `src-tauri/src/web_api_proxy.rs`, which exposes a public bridge on `0.0.0.0:${LLM_WIKI_WEB_PORT:-19830}` and forwards requests to the original local API.

This is a migration bridge, not a new knowledge-base implementation. The final serviceized version should move the original API bind address itself to `0.0.0.0` after the full command-equivalence test suite is in place.

## Environment

Copy `.env.web.example` and set `VITE_LLM_WIKI_API_BASE` to the public proxy URL when accessing the Web UI from another node.

Example:

```text
VITE_LLM_WIKI_WEB_MODE=true
VITE_LLM_WIKI_API_BASE=http://192.168.1.10:19830
LLM_WIKI_WEB_PORT=19830
```

## Current branch status

This branch currently contains the safe foundation only:

- `src/lib/http-command-client.ts`
- `src/lib/web-mode.ts`
- `src/commands/http-api.ts`
- `src/commands/web-equivalent.ts`
- `src/commands/web-fs.ts`
- `src/commands/web-projects.ts`
- `src-tauri/src/web_api_proxy.rs`
- `scripts/check-web-equivalence.mjs`
- `deploy-web.sh`
- `deploy-web.bat`

It intentionally does not include the earlier standalone RAG kernel because that violated the strict equivalence constraints.
