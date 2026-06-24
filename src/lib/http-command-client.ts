export interface HttpCommandOptions {
  baseUrl?: string
  token?: string
  signal?: AbortSignal
}

const DEFAULT_BASE_URL = "http://127.0.0.1:19828"
const API_PREFIX = "/api/v1"

function trimSlash(value: string): string {
  return value.replace(/\/+$/, "")
}

function apiBase(options?: HttpCommandOptions): string {
  return trimSlash(options?.baseUrl || import.meta.env.VITE_LLM_WIKI_API_BASE || DEFAULT_BASE_URL)
}

function authHeaders(options?: HttpCommandOptions): Record<string, string> {
  const token = options?.token || import.meta.env.VITE_LLM_WIKI_API_TOKEN || ""
  return token ? { "X-LLM-Wiki-Token": token } : {}
}

export async function httpGet<T>(path: string, options?: HttpCommandOptions): Promise<T> {
  const res = await fetch(`${apiBase(options)}${path}`, {
    method: "GET",
    headers: authHeaders(options),
    signal: options?.signal,
  })
  return parseResponse<T>(res)
}

export async function httpPost<T>(path: string, body?: unknown, options?: HttpCommandOptions): Promise<T> {
  const res = await fetch(`${apiBase(options)}${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...authHeaders(options),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: options?.signal,
  })
  return parseResponse<T>(res)
}

export async function httpPostRaw(path: string, body?: unknown, options?: HttpCommandOptions): Promise<Response> {
  return fetch(`${apiBase(options)}${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...authHeaders(options),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: options?.signal,
  })
}

async function parseResponse<T>(res: Response): Promise<T> {
  const text = await res.text()
  const data = text ? JSON.parse(text) : null
  if (!res.ok || data?.ok === false) {
    throw new Error(data?.error || `${res.status} ${res.statusText}`)
  }
  return data as T
}

export function projectApiPath(projectId: string, suffix: string): string {
  const encodedProjectId = encodeURIComponent(projectId)
  const cleanSuffix = suffix.startsWith("/") ? suffix : `/${suffix}`
  return `${API_PREFIX}/projects/${encodedProjectId}${cleanSuffix}`
}

export function healthApiPath(): string {
  return `${API_PREFIX}/health`
}
