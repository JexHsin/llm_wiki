import { healthApiPath, httpGet, httpPost, projectApiPath, type HttpCommandOptions } from "@/lib/http-command-client"

export interface ApiHealth {
  ok: boolean
  status: string
  version: string
  authRequired: boolean
  authConfigured: boolean
  tokenSource: string
  enabled: boolean
  mcpEnabled: boolean
  allowUnauthenticated: boolean
}

export interface ApiProjectEntry {
  id: string
  name: string
  path: string
  current: boolean
}

export interface ApiProjectsResponse {
  ok: boolean
  projects: ApiProjectEntry[]
  currentProject: ApiProjectEntry | null
}

export interface ApiFileNode {
  name: string
  path: string
  isDir: boolean
  size?: number
  children?: ApiFileNode[]
}

export interface ApiFilesResponse {
  ok: boolean
  projectId: string
  root: string
  files: ApiFileNode[]
  truncated: boolean
}

export interface ApiFileContentResponse {
  ok: boolean
  projectId: string
  path: string
  content: string
}

export interface ApiReviewsResponse {
  ok: boolean
  projectId: string
  status: string
  reviews: unknown[]
}

export interface ApiSearchResponse {
  ok: boolean
  projectId: string
  query: string
  results: unknown[]
}

export interface ApiGraphResponse {
  ok: boolean
  projectId: string
  nodes: unknown[]
  edges: unknown[]
}

export interface ApiRescanResponse {
  ok: boolean
  projectId: string
}

export function apiHealth(options?: HttpCommandOptions): Promise<ApiHealth> {
  return httpGet<ApiHealth>(healthApiPath(), options)
}

export function apiProjects(options?: HttpCommandOptions): Promise<ApiProjectsResponse> {
  return httpGet<ApiProjectsResponse>("/api/v1/projects", options)
}

export function apiProjectFiles(
  projectId: string,
  params: { root?: "wiki" | "sources" | "raw" | "all"; recursive?: boolean; maxFiles?: number } = {},
  options?: HttpCommandOptions,
): Promise<ApiFilesResponse> {
  const query = new URLSearchParams()
  if (params.root) query.set("root", params.root)
  if (typeof params.recursive === "boolean") query.set("recursive", String(params.recursive))
  if (typeof params.maxFiles === "number") query.set("maxFiles", String(params.maxFiles))
  const suffix = `/files${query.toString() ? `?${query}` : ""}`
  return httpGet<ApiFilesResponse>(projectApiPath(projectId, suffix), options)
}

export function apiProjectFileContent(
  projectId: string,
  path: string,
  options?: HttpCommandOptions,
): Promise<ApiFileContentResponse> {
  const query = new URLSearchParams({ path })
  return httpGet<ApiFileContentResponse>(projectApiPath(projectId, `/files/content?${query}`), options)
}

export function apiProjectReviews(
  projectId: string,
  params: { status?: "unresolved" | "resolved" | "all"; type?: string; limit?: number } = {},
  options?: HttpCommandOptions,
): Promise<ApiReviewsResponse> {
  const query = new URLSearchParams()
  if (params.status) query.set("status", params.status)
  if (params.type) query.set("type", params.type)
  if (typeof params.limit === "number") query.set("limit", String(params.limit))
  return httpGet<ApiReviewsResponse>(projectApiPath(projectId, `/reviews${query.toString() ? `?${query}` : ""}`), options)
}

export function apiProjectSearch(
  projectId: string,
  body: { query: string; maxResults?: number },
  options?: HttpCommandOptions,
): Promise<ApiSearchResponse> {
  return httpPost<ApiSearchResponse>(projectApiPath(projectId, "/search"), body, options)
}

export function apiProjectGraph(projectId: string, options?: HttpCommandOptions): Promise<ApiGraphResponse> {
  return httpGet<ApiGraphResponse>(projectApiPath(projectId, "/graph"), options)
}

export function apiProjectRescan(projectId: string, options?: HttpCommandOptions): Promise<ApiRescanResponse> {
  return httpPost<ApiRescanResponse>(projectApiPath(projectId, "/sources/rescan"), {}, options)
}

export function apiProjectChat(projectId: string, body: unknown, options?: HttpCommandOptions): Promise<unknown> {
  return httpPost(projectApiPath(projectId, "/chat"), body, options)
}
