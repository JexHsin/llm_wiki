# LLM Wiki Web RAG Migration

This branch replaces the Tauri desktop runtime with a web-first runtime.

## Runtime

Start the server:

```bash
npm start
```

Linux one-click deployment:

```bash
./deploy.sh
```

Windows one-click deployment:

```bat
deploy.bat
```

The server listens on `0.0.0.0:19828` by default so it can be reached from outside the local node.

## Web UI

Open:

```text
http://<server-ip>:19828/
```

The standalone web console supports chat and graph inspection.

## HTTP API

- `GET /health`
- `POST /api/chat`
- `POST /api/knowledge/query`
- `POST /api/vector/search`
- `GET /api/graph/entities`
- `GET /api/graph/relations`
- `POST /api/graph/query`
- `POST /api/wiki/reindex`

## RAG Pipeline

The production web runtime keeps the original LLM Wiki design principles:

1. Load wiki/raw/docs/content markdown and text files.
2. Parse titles and `[[wikilink]]` relations.
3. Build a graph from pages and links.
4. Chunk wiki pages.
5. Build deterministic local embeddings for zero-dependency vector search.
6. Retrieve top-k context.
7. Expand graph context.
8. Build a chat answer with optional OpenAI-compatible LLM.

## LLM Configuration

Without credentials the server returns local RAG results with citations. To enable LLM generation, configure:

```bash
export LLM_API_KEY=...
export LLM_BASE_URL=https://api.openai.com/v1
export LLM_MODEL=gpt-4o-mini
```

Any OpenAI-compatible endpoint can be used.

## Tests

Run:

```bash
npm test
```

The smoke test creates a temporary wiki, builds the vector index and graph, runs retrieval, and verifies chat output.
