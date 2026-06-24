import type { FileNode, WikiProject } from "@/types/wiki"
import {
  apiProjectFileContent,
  apiProjectFiles,
  apiProjectGraph,
  apiProjectReviews,
  apiProjectSearch,
  apiProjects,
  type ApiFileNode,
} from "@/commands/http-api"
import type { HttpCommandOptions } from "@/lib/http-command-client"

export function toWikiProject(project: { id: string; name: string; path: string }): WikiProject {
  return {
    id: project.id,
    name: project.name,
    path: project.path,
  }
}

export function toFileNode(node: ApiFileNode): FileNode {
  return {
    name: node.name,
    path: node.path,
    is_dir: node.isDir,
    children: node.children?.map(toFileNode),
  }
}

export async function listWebProjects(options?: HttpCommandOptions): Promise<WikiProject[]> {
  const res = await apiProjects(options)
  return res.projects.map(toWikiProject)
}

export async function getCurrentWebProject(options?: HttpCommandOptions): Promise<WikiProject | null> {
  const res = await apiProjects(options)
  return res.currentProject ? toWikiProject(res.currentProject) : null
}

export async function listWebProjectFiles(
  projectId = "current",
  params: { root?: "wiki" | "sources" | "raw" | "all"; recursive?: boolean; maxFiles?: number } = {},
  options?: HttpCommandOptions,
): Promise<FileNode[]> {
  const res = await apiProjectFiles(projectId, params, options)
  return res.files.map(toFileNode)
}

export async function readWebProjectTextFile(
  projectId: string,
  relativePath: string,
  options?: HttpCommandOptions,
): Promise<string> {
  const res = await apiProjectFileContent(projectId, relativePath, options)
  return res.content
}

export async function searchWebProject(
  projectId: string,
  query: string,
  maxResults?: number,
  options?: HttpCommandOptions,
): Promise<unknown[]> {
  const res = await apiProjectSearch(projectId, { query, topK: maxResults }, options)
  return res.results
}

export async function getWebProjectGraph(projectId = "current", options?: HttpCommandOptions) {
  return apiProjectGraph(projectId, options)
}

export async function getWebProjectReviews(projectId = "current", options?: HttpCommandOptions) {
  return apiProjectReviews(projectId, { status: "all" }, options)
}
