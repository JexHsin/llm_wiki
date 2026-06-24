async function j(url, body) {
  const opt = body ? { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) } : undefined;
  return fetch(url, opt).then(r => r.json());
}

window.sendChat = async function() {
  const body = {
    session_id: document.getElementById('session').value || 'default',
    message: document.getElementById('message').value,
    top_k: 8
  };
  document.getElementById('output').textContent = 'loading';
  const ans = await j('/api/chat', body);
  document.getElementById('output').textContent = JSON.stringify(ans, null, 2);
};

window.refreshGraph = async function(){
  const h = await j('/health');
  const n = await j('/api/graph/entities');
  const e = await j('/api/graph/relations');
  document.getElementById('stats').textContent = JSON.stringify(h, null, 2);
  document.getElementById('nodes').textContent = JSON.stringify(n, null, 2);
  document.getElementById('edges').textContent = JSON.stringify(e, null, 2);
};

window.refreshGraph();
