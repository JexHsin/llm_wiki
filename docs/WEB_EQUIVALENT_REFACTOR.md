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
- Use `src/commands/web-equivalent.ts`, `src/commands/web-fs.ts`, `src/commands/web-projects.ts`, and `src/commands/web-graph.ts` as adapter façades that convert API payloads back to existing frontend domain types.

### Completed substitutions

When `VITE_LLM_WIKI_WEB_MODE=true`, the following paths now use HTTP instead of direct Tauri invoke calls:

- `src/commands/fs.ts`
  - `readFile` uses project-scoped `/files/read` and forwards `extractImages`, so `.llm-wiki/*`, PDF/Office preprocessing reads, media placeholders, and extracted-image behavior keep the same Rust `commands::fs::read_file` semantics as desktop mode.
  - `writeFile`
  - `writeFileBase64`
  - `writeFileAtomic`
  - `readFileAsBase64`
  - `copyFile`
  - `copyDirectory`
  - `preprocessFile`
  - `deleteFile`
  - `createDirectory`
  - `fileExists`
  - `getFileModifiedTime`
  - `getFileSize`
  - `getFileMd5`
  - `findRelatedWikiPages`
  - `createProject`
  - `listDirectory` maps API file trees back to the requested subtree, matching desktop `list_directory(path)` semantics.
  - `openProject`
  - `apiServerStatus`
- `src/lib/embedding.ts`
  - The original chunking, embedding prompt enrichment, retry/auto-halving, rebuild semantics, page aggregation, and matched-chunk scoring are preserved.
  - Web mode routes embedding provider HTTP requests through the authenticated backend proxy so CORS-unfriendly embedding endpoints behave like the original Tauri HTTP plugin path.
  - Web mode routes LanceDB vector operations through HTTP while still delegating to the original Rust `commands::vectorstore::*` functions: chunk upsert/search/delete/count/clear/optimize and legacy v1 count/drop.
- `src/lib/llm-client.ts`
  - `streamChat` still uses the original `llm-providers.ts` provider URL/body/header/parse pipeline. In Web mode, the provider HTTP request is sent through `/chat` as an authenticated proxy instead of being reimplemented in Rust.
- `src/lib/search.ts`
  - `searchWiki` uses the existing `/search` API with the original `topK: 20`, `includeContent: false`, and `queryEmbedding: null` contract.
- `src/lib/wiki-graph.ts`
  - `buildWikiGraph` delegates to `src/commands/web-graph.ts`, which reads the existing `/graph` API via `getWebProjectGraph()` / `apiProjectGraph()` and normalizes node paths back to absolute project paths.
- `src/lib/persist.ts`
  - Review, Lint, and Chat history persistence continue to read/write the same `.llm-wiki/*.json` files through the Web filesystem façade. No Review/Lint data format or limit/sanitization is introduced in the frontend persistence path.
- `src/components/project/welcome-screen.tsx`
  - recent/current project list uses the existing `/projects` API through `src/commands/web-projects.ts`.
- `src/commands/web-projects.ts`
  - project list/current project helpers are available for the project selection UI migration.

Desktop mode remains unchanged unless `VITE_LLM_WIKI_WEB_MODE=true` is set.

### Web-mode safeguards

Project-scoped read/write/copy/metadata/preprocess/vector endpoints are implemented in the Web proxy. Compatibility endpoints follow the same API enabled/auth rules as the original local API server. Project file paths are resolved against the current project, reject traversal/prefix/root escapes, and canonicalize existing paths plus the nearest existing ancestor for new paths to reduce symlink escape risk. Unsupported desktop shell operations still fail fast in Web mode instead of falling through to Tauri.

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

The original Rust API remains the source of truth on `127.0.0.1:19828`. To avoid rewriting or duplicating business logic, this branch adds `src-tauri/src/web_api_proxy.rs`, which exposes a public bridge on `0.0.0.0:${LLM_WIKI_WEB_PORT:-19830}` and forwards to the original local API.

The bridge also exposes compatibility endpoints that the original local API did not yet include:

- `POST /api/v1/projects/create`
- `GET /api/v1/projects/{projectId}/lint`
- `POST /api/v1/projects/{projectId}/files/read`
- `POST /api/v1/projects/{projectId}/files/read-base64`
- `POST /api/v1/projects/{projectId}/files/write`
- `POST /api/v1/projects/{projectId}/files/write-base64`
- `POST /api/v1/projects/{projectId}/files/write-atomic`
- `POST /api/v1/projects/{projectId}/files/delete`
- `POST /api/v1/projects/{projectId}/files/copy`
- `POST /api/v1/projects/{projectId}/files/preprocess`
- `POST /api/v1/projects/{projectId}/files/metadata`
- `POST /api/v1/projects/{projectId}/directories/create`
- `POST /api/v1/projects/{projectId}/directories/copy`
- `POST /api/v1/projects/{projectId}/wiki/related-pages`
- `POST /api/v1/projects/{projectId}/vectors/chunks/upsert`
- `POST /api/v1/projects/{projectId}/vectors/chunks/search`
- `POST /api/v1/projects/{projectId}/vectors/pages/delete`
- `POST /api/v1/projects/{projectId}/vectors/chunks/count`
- `POST /api/v1/projects/{projectId}/vectors/chunks/clear`
- `POST /api/v1/projects/{projectId}/vectors/chunks/optimize`
- `POST /api/v1/projects/{projectId}/vectors/legacy/count`
- `POST /api/v1/projects/{projectId}/vectors/legacy/drop`
- `POST /api/v1/projects/{projectId}/chat`

This is a migration bridge, not a new knowledge-base implementation. The final serviceized version should move these endpoints into the original API server once full command-equivalence tests are in place.

## Remaining non-equivalent / desktop-only operations

The remaining explicit Web-mode failures are desktop shell or local integration operations rather than core Wiki business logic:

- `openProjectFolder` — opens the host OS file explorer.
- `apiServerReloadConfig` — directly reloads the local desktop API config cache.
- `mcpServerEntryPath` — exposes a local desktop-side MCP entry path.
- `clipServerStatus` — still uses the desktop command path.

Full desktop-vs-web equivalence tests still need to be run before marking the PR ready.

## Environment

Copy `.env.web.example` and set `VITE_LLM_WIKI_API_BASE` to the public proxy URL when accessing the Web UI from another node.

Example:

```text
VITE_LLM_WIKI_WEB_MODE=true
VITE_LLM_WIKI_API_BASE=http://192.168.1.10:19830
LLM_WIKI_WEB_PORT=19830
```

## Current branch status

This branch currently contains the strict Web-equivalence foundation and compatibility API bridge. It intentionally does not include the earlier standalone RAG kernel because that violated the strict equivalence constraints.
