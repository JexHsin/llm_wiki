import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const forbiddenNewFrameworks = [
  'lancedb-node',
  'chromadb',
  'langchain',
  'llamaindex',
  'vectordb',
];
const forbiddenStandaloneKernelPaths = [
  'web-server/rag-kernel.mjs',
  'web-server/server.mjs',
];

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(root, file), 'utf8'));
}

const pkg = readJson('package.json');
const allDeps = {
  ...(pkg.dependencies || {}),
  ...(pkg.devDependencies || {}),
};

for (const dep of forbiddenNewFrameworks) {
  if (dep in allDeps) {
    throw new Error(`New knowledge-base framework is not allowed: ${dep}`);
  }
}

for (const p of forbiddenStandaloneKernelPaths) {
  if (fs.existsSync(path.join(root, p))) {
    throw new Error(`Standalone RAG kernel is not allowed in web-equivalent migration: ${p}`);
  }
}

const requiredPaths = [
  'src/lib/http-command-client.ts',
  'src-tauri/src/api_server.rs',
  'src-tauri/src/web_api_proxy.rs',
  'src/commands/fs.ts',
  'src/App.tsx',
  'deploy-web.sh',
  'deploy-web.bat',
];

for (const p of requiredPaths) {
  if (!fs.existsSync(path.join(root, p))) {
    throw new Error(`Missing required migration path: ${p}`);
  }
}

console.log('web equivalence guard passed');
