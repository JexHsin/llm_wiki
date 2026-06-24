import { load } from "@tauri-apps/plugin-store"
import type { WikiProject } from "@/types/wiki"
import type { ApiConfig, GeneralConfig, LlmConfig, SearchApiConfig, EmbeddingConfig, MineruConfig, MultimodalConfig, OutputLanguage, ProviderConfigs, ProxyConfig, ScheduledImportConfig, SourceWatchConfig } from "@/stores/wiki-store"
import { normalizeSourceWatchConfig } from "@/lib/source-watch-config"
import { normalizePath } from "@/lib/path-utils"
import { DEFAULT_ZOOM_LEVEL, clampZoomLevel } from "@/stores/zoom-store"
import { isWebMode } from "@/lib/web-mode"
import { getWebLastProject, getWebRecentProjects } from "@/commands/web-projects"

const STORE_NAME = "app-state.json"
const RECENT_PROJECTS_KEY = "recentProjects"
const LAST_PROJECT_KEY = "lastProject"

async function getStore() {
  return load(STORE_NAME, { autoSave: true, defaults: {} })
}

export async function getRecentProjects(): Promise<WikiProject[]> {
  if (isWebMode()) return getWebRecentProjects()
  const store = await getStore()
  const projects = await store.get<WikiProject[]>(RECENT_PROJECTS_KEY)
  return projects ?? []
}

export async function getLastProject(): Promise<WikiProject | null> {
  if (isWebMode()) return getWebLastProject()
  const store = await getStore()
  const project = await store.get<WikiProject>(LAST_PROJECT_KEY)
  return project ?? null
}

export async function saveLastProject(project: WikiProject): Promise<void> {
  const store = await getStore()
  await store.set(LAST_PROJECT_KEY, project)
  await addToRecentProjects(project)
}

export async function addToRecentProjects(
  project: WikiProject
): Promise<void> {
  const store = await getStore()
  const existing = (await store.get<WikiProject[]>(RECENT_PROJECTS_KEY)) ?? []
  const filtered = existing.filter((p) => p.path !== project.path)
  const updated = [project, ...filtered].slice(0, 10)
  await store.set(RECENT_PROJECTS_KEY, updated)
}

const LLM_CONFIG_KEY = "llmConfig"
const PROVIDER_CONFIGS_KEY = "providerConfigs"
const ACTIVE_PRESET_KEY = "activePresetId"

export async function saveLlmConfig(config: LlmConfig): Promise<void> {
  const store = await getStore()
  await store.set(LLM_CONFIG_KEY, config)
}

export async function loadLlmConfig(): Promise<LlmConfig | null> {
  const store = await getStore()
  return (await store.get<LlmConfig>(LLM_CONFIG_KEY)) ?? null
}

export async function saveProviderConfigs(configs: ProviderConfigs): Promise<void> {
  const store = await getStore()
  await store.set(PROVIDER_CONFIGS_KEY, configs)
}

export async function loadProviderConfigs(): Promise<ProviderConfigs | null> {
  const store = await getStore()
  return (await store.get<ProviderConfigs>(PROVIDER_CONFIGS_KEY)) ?? null
}

export async function saveActivePresetId(id: string | null): Promise<void> {
  const store = await getStore()
  await store.set(ACTIVE_PRESET_KEY, id)
}

export async function loadActivePresetId(): Promise<string | null> {
  const store = await getStore()
  return (await store.get<string | null>(ACTIVE_PRESET_KEY)) ?? null
}

const SEARCH_API_KEY = "searchApiConfig"

export async function saveSearchApiConfig(config: SearchApiConfig): Promise<void> {
  const store = await getStore()
  await store.set(SEARCH_API_KEY, config)
}

export async function loadSearchApiConfig(): Promise<SearchApiConfig | null> {
  const store = await getStore()
  return (await store.get<SearchApiConfig>(SEARCH_API_KEY)) ?? null
}

const EMBEDDING_KEY = "embeddingConfig"

export async function saveEmbeddingConfig(config: EmbeddingConfig): Promise<void> {
  const store = await getStore()
  await store.set(EMBEDDING_KEY, config)
}

export async function loadEmbeddingConfig(): Promise<EmbeddingConfig | null> {
  const store = await getStore()
  return (await store.get<EmbeddingConfig>(EMBEDDING_KEY)) ?? null
}

const MULTIMODAL_KEY = "multimodalConfig"

export async function saveMultimodalConfig(config: MultimodalConfig): Promise<void> {
  const store = await getStore()
  await store.set(MULTIMODAL_KEY, config)
}

export async function loadMultimodalConfig(): Promise<MultimodalConfig | null> {
  const store = await getStore()
  return (await store.get<MultimodalConfig>(MULTIMODAL_KEY)) ?? null
}

const MINERU_KEY = "mineruConfig"

function normalizeMineruConfig(config: MineruConfig): MineruConfig {
  return {
    enabled: config.enabled === true,
    token: typeof config.token === "string" ? config.token : "",
    modelVersion: config.modelVersion === "pipeline" ? "pipeline" : "vlm",
  }
}

function normalizeZoomLevel(level: unknown): number {
  return typeof level === "number" && Number.isFinite(level)
    ? clampZoomLevel(level)
    : DEFAULT_ZOOM_LEVEL
}

export const __projectStoreTest = {
  normalizeMineruConfig,
  normalizeZoomLevel,
}

export async function saveMineruConfig(config: MineruConfig): Promise<void> {
  const store = await getStore()
  await store.set(MINERU_KEY, normalizeMineruConfig(config))
}

export async function loadMineruConfig(): Promise<MineruConfig | null> {
  const store = await getStore()
  const raw = await store.get<MineruConfig>(MINERU_KEY)
  return raw ? normalizeMineruConfig(raw) : null
}
