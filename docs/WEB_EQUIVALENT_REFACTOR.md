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

## Current branch status

This branch currently contains the safe foundation only:

- `src/lib/http-command-client.ts`
- `scripts/check-web-equivalence.mjs`
- `deploy-web.sh`
- `deploy-web.bat`

It intentionally does not include the earlier standalone RAG kernel because that violated the strict equivalence constraints.
