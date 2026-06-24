import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const DIM = 512;

export class MemoryStore {
  constructor() {
    this.sessions = new Map();
  }
  add(sessionId, role, content) {
    const sid = sessionId || 'default';
    const arr = this.sessions.get(sid) || [];
    arr.push({ role, content, ts: new Date().toISOString() });
    this.sessions.set(sid, arr.slice(-50));
  }
  get(sessionId) {
    return this.sessions.get(sessionId || 'default') || [];
  }
}

export class GraphStore {
  constructor() {
    this.nodes = new Map();
    this.edges = [];
  }
  addNode(id, data = {}) {
    this.nodes.set(id, { id, ...data });
  }
  addEdge(source, target, label = 'wikilink') {
    if (!source || !target) return;
    this.edges.push({ source, target, label });
  }
  expand(seedIds = [], depth = 1) {
    const seen = new Set(seedIds);
    let frontier = [...seedIds];
    for (let i = 0; i < depth; i++) {
      const next = [];
      for (const e of this.edges) {
        if (frontier.includes(e.source) && !seen.has(e.target)) {
          seen.add(e.target); next.push(e.target);
        }
        if (frontier.includes(e.target) && !seen.has(e.source)) {
          seen.add(e.source); next.push(e.source);
        }
      }
      frontier = next;
    }
    return {
      nodes: [...seen].map((id) => this.nodes.get(id)).filter(Boolean),
      edges: this.edges.filter((e) => seen.has(e.source) && seen.has(e.target)),
    };
  }
}

function tokenize(text) {
  return String(text || '')
    .toLowerCase()
    .replace(/[^\p{L}\p{N}_\-\s]/gu, ' ')
    .split(/\s+/)
    .filter(Boolean);
}

function hashToken(token) {
  const h = crypto.createHash('sha256').update(token).digest();
  return h.readUInt32BE(0);
}

export function embedLocal(text) {
  const vec = new Array(DIM).fill(0);
  for (const token of tokenize(text)) {
    const h = hashToken(token);
    const idx = h % DIM;
    vec[idx] += (h & 1) ? 1 : -1;
  }
  const norm = Math.sqrt(vec.reduce((s, v) => s + v * v, 0)) || 1;
  return vec.map((v) => v / norm);
}

function cosine(a, b) {
  let s = 0;
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) s += a[i] * b[i];
  return s;
}

export class VectorIndex {
  constructor() {
    this.items = [];
  }
  add(item) {
    this.items.push({ ...item, vector: item.vector || embedLocal(item.text) });
  }
  search(query, topK = 8) {
    const q = Array.isArray(query) ? query : embedLocal(query);
    return this.items
      .map((it) => ({ ...it, score: cosine(q, it.vector) }))
      .sort((a, b) => b.score - a.score)
      .slice(0, topK)
      .map(({ vector, ...rest }) => rest);
  }
}

function stripFrontmatter(text) {
  return String(text || '').replace(/^---[\s\S]*?---\s*/m, '');
}

function titleFromMarkdown(filePath, text) {
  const m = String(text).match(/^#\s+(.+)$/m);
  return m ? m[1].trim() : path.basename(filePath, path.extname(filePath));
}

function wikilinks(text) {
  const links = [];
  const re = /\[\[([^\]]+)\]\]/g;
  let m;
  while ((m = re.exec(text))) links.push(m[1].split('|')[0].trim());
  return links;
}

function chunkText(text, size = 1400, overlap = 180) {
  const clean = stripFrontmatter(text).replace(/\r\n/g, '\n');
  const chunks = [];
  for (let start = 0; start < clean.length; start += size - overlap) {
    const part = clean.slice(start, start + size).trim();
    if (part) chunks.push(part);
  }
  return chunks;
}

function walk(dir) {
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const name of fs.readdirSync(dir)) {
    const p = path.join(dir, name);
    const st = fs.statSync(p);
    if (st.isDirectory()) out.push(...walk(p));
    else if (/\.(md|markdown|txt)$/i.test(name)) out.push(p);
  }
  return out;
}

export class WikiStore {
  constructor(rootDir) {
    this.rootDir = rootDir;
    this.pages = [];
  }
  load() {
    const candidates = ['wiki', 'raw', 'docs', 'content'].map((d) => path.join(this.rootDir, d));
    const files = candidates.flatMap(walk);
    this.pages = files.map((file) => {
      const text = fs.readFileSync(file, 'utf8');
      const rel = path.relative(this.rootDir, file);
      return { id: rel.replace(/\\/g, '/'), path: rel, title: titleFromMarkdown(file, text), text, links: wikilinks(text) };
    });
    return this.pages;
  }
}

export class RagEngine {
  constructor({ rootDir = process.cwd() } = {}) {
    this.rootDir = rootDir;
    this.memory = new MemoryStore();
    this.graph = new GraphStore();
    this.vector = new VectorIndex();
    this.wiki = new WikiStore(rootDir);
  }
  build() {
    const pages = this.wiki.load();
    this.graph = new GraphStore();
    this.vector = new VectorIndex();
    for (const page of pages) {
      this.graph.addNode(page.id, { title: page.title, path: page.path, type: 'wiki_page' });
      for (const link of page.links) this.graph.addEdge(page.id, link, 'wikilink');
      chunkText(page.text).forEach((chunk, i) => this.vector.add({ id: `${page.id}#${i}`, pageId: page.id, title: page.title, text: chunk }));
    }
    return { pages: pages.length, chunks: this.vector.items.length, relations: this.graph.edges.length };
  }
  queryKnowledge(question, topK = 8) {
    const hits = this.vector.search(question, topK);
    const seeds = [...new Set(hits.map((h) => h.pageId))];
    return { hits, graph: this.graph.expand(seeds, 1) };
  }
  async chat({ session_id = 'default', message = '', top_k = 8 }) {
    this.memory.add(session_id, 'user', message);
    const ctx = this.queryKnowledge(message, top_k);
    const answer = await this.answerWithOptionalLlm(message, ctx, this.memory.get(session_id));
    this.memory.add(session_id, 'assistant', answer);
    return { answer, context: ctx.hits, graph: ctx.graph, memory: this.memory.get(session_id) };
  }
  async answerWithOptionalLlm(question, ctx, history) {
    const apiKey = process.env.LLM_API_KEY || process.env.OPENAI_API_KEY;
    const baseUrl = process.env.LLM_BASE_URL || 'https://api.openai.com/v1';
    const model = process.env.LLM_MODEL || 'gpt-4o-mini';
    const contextText = ctx.hits.map((h, i) => `[${i + 1}] ${h.title}\n${h.text}`).join('\n\n');
    if (!apiKey) {
      const sources = ctx.hits.slice(0, 5).map((h, i) => `${i + 1}. ${h.title} (${h.pageId})`).join('\n');
      return `未配置 LLM_API_KEY，已返回本地 RAG 检索结果。\n\n问题：${question}\n\n参考来源：\n${sources || '未检索到相关内容'}`;
    }
    const messages = [
      { role: 'system', content: 'You are LLM Wiki. Answer strictly from the provided wiki context when possible. Cite source titles.' },
      ...history.slice(-8).map((m) => ({ role: m.role, content: m.content })),
      { role: 'user', content: `Question:\n${question}\n\nWiki context:\n${contextText}` },
    ];
    const res = await fetch(`${baseUrl.replace(/\/$/, '')}/chat/completions`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: `Bearer ${apiKey}` },
      body: JSON.stringify({ model, messages, temperature: 0.2 }),
    });
    if (!res.ok) return `LLM 调用失败：${res.status} ${await res.text()}`;
    const json = await res.json();
    return json.choices?.[0]?.message?.content || '';
  }
}
