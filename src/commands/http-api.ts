import { healthApiPath, httpGet, httpPost, httpPostRaw, projectApiPath, type HttpCommandOptions } from "@/lib/http-command-client"

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

export interface ApiCreateProjectResponse {
  ok: boolean
  name: string
  path: string
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

export interface ApiFileBase64Response {
  ok: boolean
  projectId: string
  path: string
  base64: string
  mimeType: string
}

export interface ApiWriteResponse {
  ok: boolean
  projectId: string
  path: string
}

export interface ApiCopyDirectoryResponse {
  ok: boolean
  projectId: string
  files: string[]
}

export interface ApiFileMetadataResponse {
  ok: boolean
  projectId: string
  path: string
  modifiedTime: number
  size: number
  md5: string
}

export interface ApiRelatedWikiPagesResponse {
  ok: boolean
  projectId: string
  pages: string[]
}

export interface ApiPreprocessResponse {
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

export interface ApiLintResponse {
  ok: boolean
  projectId: string
  count: number
  lint: unknown[]
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

export interface ApiProviderChatRequest {
  url: string
  headers: Record<string, string>
  body: unknown
}

export function apiHealth(options?: HttpCommandOptions): Promise<ApiHealth> {
  return httpGet<ApiHealth>(healthApiPath(), options)
}

export function apiProjects(options?: HttpCommandOptions): Promise<ApiProjectsResponse> {
  return httpGet<ApiProjectsResponse>("/api/v1/projects", options)
}

export function apiCreateProject(
  name: string,
  path: string,
  options?: HttpCommandOptions,
): Promise<ApiCreateProjectResponse> {
  return httpPost<ApiCreateProjectResponse>("/api/v1/projects/create", { name, path }, options)
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

export function apiProjectReadFile(
  projectId: string,
  path: string,
  options?: HttpCommandOptions,
): Promise<ApiFileContentResponse> {
  return httpPost<ApiFileContentResponse>(projectApiPath(projectId, "/files/read"), { path }, options)
}

export function apiProjectReadFileBase64(
  projectId: string,
  path: string,
  options?: HttpCommandOptions,
): Promise<ApiFileBase64Response> {
  return httpPost<ApiFileBase64Response>(projectApiPath(projectId, "/files/read-base64"), { path }, options)
}

export function apiProjectWriteFile(
  projectId: string,
  path: string,
  contents: string,
  options?: HttpCommandOptions,
): Promise<ApiWriteResponse> {
  return httpPost<ApiWriteResponse>(projectApiPath(projectId, "/files/write"), { path, contents }, options)
}

export function apiProjectWriteFileBase64(
  projectId: string,
  path: string,
  base64: string,
  options?: HttpCommandOptions,
): Promise<ApiWriteResponse> {
  return httpPost<ApiWriteResponse>(projectApiPath(projectId, "/files/write-base64"), { path, base64 }, options)
}

export function apiProjectWriteFileAtomic(
  projectId: string,
  path: string,
  contents: string,
  options?: HttpCommandOptions,
): Promise<ApiWriteResponse> {
  return httpPost<ApiWriteResponse>(projectApiPath(projectId, "/files/write-atomic"), { path, contents }, options)
}

export function apiProjectDeleteFile(
  projectId: string,
  path: string,
  options?: HttpCommandOptions,
): Promise<ApiWriteResponse> {
  return httpPost<ApiWriteResponse>(projectApiPath(projectId, "/files/delete"), { path }, options)
}

export function apiProjectCreateDirectory(
  projectId: string,
  path: string,
  options?: HttpCommandOptions,
): Promise<ApiWriteResponse> {
  return httpPost<ApiWriteResponse>(projectApiPath(projectId, "/directories/create"), { path }, options)
}

export function apiProjectCopyFile(
  projectId: string,
  source: string,
  destination: string,
  options?: HttpCommandOptions,
): Promise<ApiWriteResponse> {
  return httpPost<ApiWriteResponse>(projectApiPath(projectId, "/files/copy"), { source, destination }, options)
}

export function apiProjectCopyDirectory(
  projectId: string,
  source: string,
  destination: string,
  options?: HttpCommandOptions,
): Promise<ApiCopyDirectoryResponse> {
  return httpPost<ApiCopyDirectoryResponse>(projectApiPath(projectId, "/directories/copy"), { source, destination }, options)
}

export function apiProjectPreprocessFile(
  projectId: string,
  path: string,
  options?: HttpCommandOptions,
): Promise<ApiPreprocessResponse> {
  return httpPost<ApiPreprocessResponse>(projectApiPath(projectId, "/files/preprocess"), { path }, options)
}

export function apiProjectFileMetadata(
  projectId: string,
  path: string,
  options?: HttpCommandOptions,
): Promise<ApiFileMetadataResponse> {
  return httpPost<ApiFileMetadataResponse>(projectApiPath(projectId, "/files/metadata"), { path }, options)
}

export function apiProjectRelatedWikiPages(
  projectId: string,
  sourceName: string,
  options?: HttpCommandOptions,
): Promise<ApiRelatedWikiPagesResponse> {
  return httpPost<ApiRelatedWikiPagesResponse>(projectApiPath(projectId, "/wiki/related-pages"), { sourceName }, options)
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

export function apiProjectLint(
  projectId: string,
  params: { type?: string; limit?: number } = {},
  options?: HttpCommandOptions,
): Promise<ApiLintResponse> {
  const query = new URLSearchParams()
  if (params.type) query.set("type", params.type)
  if (typeof params.limit === "number") query.set("limit", String(params.limit))
  return httpGet<ApiLintResponse>(projectApiPath(projectId, `/lint${query.toString() ? `?${query}` : ""}`), options)
}

export function apiProjectSearch(
  projectId: string,
  body: { query: string; topK?: number; includeContent?: boolean; queryEmbedding?: number[] | null },
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

export function apiProjectChat(projectId: string, body: ApiProviderChatRequest, options?: HttpCommandOptions): Promise<Response> {
  return httpPostRaw(projectApiPath(projectId, "/chat"), body, options)
}
