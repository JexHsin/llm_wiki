import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { RagEngine } from './rag-kernel.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = process.env.LLM_WIKI_ROOT || path.resolve(__dirname, '..');
const host = process.env.LLM_WIKI_HOST || '0.0.0.0';
const port = Number(process.env.LLM_WIKI_PORT || 19828);
const engine = new RagEngine({ rootDir });
let stats = engine.build();

function send(res, status, body, headers = {}) {
  const data = typeof body === 'string' ? body : JSON.stringify(body);
  res.writeHead(status, {
    'content-type': typeof body === 'string' ? 'text/plain; charset=utf-8' : 'application/json; charset=utf-8',
    'access-control-allow-origin': '*',
    'access-control-allow-methods': 'GET,POST,OPTIONS',
    'access-control-allow-headers': 'content-type,authorization',
    ...headers,
  });
  res.end(data);
}

async function readJson(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  const text = Buffer.concat(chunks).toString('utf8');
  if (!text) return {};
  return JSON.parse(text);
}

function staticFile(req, res) {
  const url = new URL(req.url, 'http://localhost');
  const dist = path.join(rootDir, 'dist');
  let target = path.join(dist, decodeURIComponent(url.pathname));
  if (!target.startsWith(dist)) return send(res, 403, 'forbidden');
  if (fs.existsSync(target) && fs.statSync(target).isDirectory()) target = path.join(target, 'index.html');
  if (!fs.existsSync(target)) target = path.join(dist, 'index.html');
  if (!fs.existsSync(target)) return send(res, 404, 'web build not found; run npm run build:web first');
  const ext = path.extname(target).toLowerCase();
  const type = ext === '.html' ? 'text/html; charset=utf-8' : ext === '.js' ? 'application/javascript; charset=utf-8' : ext === '.css' ? 'text/css; charset=utf-8' : 'application/octet-stream';
  res.writeHead(200, { 'content-type': type });
  fs.createReadStream(target).pipe(res);
}

async function route(req, res) {
  if (req.method === 'OPTIONS') return send(res, 204, '');
  const url = new URL(req.url, 'http://localhost');
  try {
    if (req.method === 'GET' && url.pathname === '/health') return send(res, 200, { ok: true, stats });
    if (req.method === 'POST' && url.pathname === '/api/wiki/reindex') {
      stats = engine.build();
      return send(res, 200, { ok: true, stats });
    }
    if (req.method === 'POST' && url.pathname === '/api/chat') {
      const body = await readJson(req);
      return send(res, 200, await engine.chat(body));
    }
    if (req.method === 'POST' && url.pathname === '/api/knowledge/query') {
      const body = await readJson(req);
      return send(res, 200, engine.queryKnowledge(body.query || body.message || '', body.top_k || 8));
    }
    if (req.method === 'POST' && url.pathname === '/api/vector/search') {
      const body = await readJson(req);
      return send(res, 200, { hits: engine.vector.search(body.query || '', body.top_k || 8) });
    }
    if (req.method === 'GET' && url.pathname === '/api/graph/entities') {
      return send(res, 200, { nodes: [...engine.graph.nodes.values()] });
    }
    if (req.method === 'GET' && url.pathname === '/api/graph/relations') {
      return send(res, 200, { edges: engine.graph.edges });
    }
    if (req.method === 'POST' && url.pathname === '/api/graph/query') {
      const body = await readJson(req);
      return send(res, 200, engine.graph.expand(body.seed_ids || body.seeds || [], body.depth || 1));
    }
    return staticFile(req, res);
  } catch (err) {
    return send(res, 500, { error: String(err?.stack || err) });
  }
}

http.createServer(route).listen(port, host, () => {
  console.log(`LLM Wiki web API listening on http://${host}:${port}`);
  console.log(`root=${rootDir}`);
});
