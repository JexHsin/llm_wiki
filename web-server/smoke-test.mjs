import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { RagEngine } from './rag-kernel.mjs';

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'llm-wiki-'));
fs.mkdirSync(path.join(root, 'wiki'), { recursive: true });
fs.writeFileSync(path.join(root, 'wiki', 'index.md'), '# Index\n\nThis wiki explains Alpha and links to [[Beta]].', 'utf8');
fs.writeFileSync(path.join(root, 'wiki', 'Beta.md'), '# Beta\n\nBeta is a graph node used by the knowledge base.', 'utf8');

const engine = new RagEngine({ rootDir: root });
const stats = engine.build();
assert.equal(stats.pages, 2);
assert.equal(stats.relations, 1);

const result = engine.queryKnowledge('Alpha Beta graph', 3);
assert.ok(result.hits.length > 0);
assert.ok(result.graph.nodes.length > 0);

const chat = await engine.chat({ session_id: 't1', message: 'What is Beta?' });
assert.ok(chat.answer.includes('Beta') || chat.context.length > 0);

console.log('RAG smoke tests passed');
