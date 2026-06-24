import type { FileNode, WikiProject } from "@/types/wiki"
import { apiProjectFileContent, apiProjectFiles, apiProjects } from "@/commands/http-api"
import { toFileNode, toWikiProject } from "@/commands/web-equivalent"
import type { HttpCommandOptions } from "@/lib/http-command-client"

export async function openCurrentWebProject(options?: HttpCommandOptions): Promise<WikiProject> {
  const res = await apiProjects(options)
  if (!res.currentProject) {
    throw new Error("No current project is available from the Web API")
  }
  return toWikiProject(res.currentProject)
}

export async function listCurrentWebProjectDirectory(
  params: { root?: "wiki" | "sources" | "raw" | "all"; recursive?: boolean; maxFiles?: number } = {},
  options?: HttpCommandOptions,
): Promise<FileNode[]> {
  const res = await apiProjectFiles("current", params, options)
  return res.files.map(toFileNode)
}

export async function readCurrentWebProjectFile(relativePath: string, options?: HttpCommandOptions): Promise<string> {
  const res = await apiProjectFileContent("current", relativePath, options)
  return res.content
}

export async function listWebWikiTree(options?: HttpCommandOptions): Promise<FileNode[]> {
  return listCurrentWebProjectDirectory({ root: "wiki", recursive: true }, options)
}

export async function listWebSourcesTree(options?: HttpCommandOptions): Promise<FileNode[]> {
  return listCurrentWebProjectDirectory({ root: "sources", recursive: true }, options)
}
