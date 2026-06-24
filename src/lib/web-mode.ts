export function isWebMode(): boolean {
  return import.meta.env.VITE_LLM_WIKI_WEB_MODE === "true"
}
