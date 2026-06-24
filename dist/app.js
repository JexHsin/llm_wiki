window.refreshGraph = async function(){
  const h = await fetch('/health').then(r=>r.json());
  document.getElementById('stats').textContent = JSON.stringify(h,null,2);
};
window.refreshGraph();
