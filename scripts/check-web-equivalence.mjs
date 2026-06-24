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

const requiredPaths = [
  'src/lib/http-command-client.ts',
  'src-tauri/src/api_server.rs',
  'src/commands/fs.ts',
  'src/App.tsx',
];

for (const p of requiredPaths) {
  if (!fs.existsSync(path.join(root, p))) {
    throw new Error(`Missing required migration path: ${p}`);
  }
}

console.log('web equivalence guard passed');
