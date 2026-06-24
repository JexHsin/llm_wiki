import type { WikiProject } from "@/types/wiki"
import { apiProjects } from "@/commands/http-api"
import { toWikiProject } from "@/commands/web-equivalent"
import type { HttpCommandOptions } from "@/lib/http-command-client"

export async function getWebRecentProjects(options?: HttpCommandOptions): Promise<WikiProject[]> {
  const res = await apiProjects(options)
  return res.projects.map(toWikiProject)
}

export async function getWebLastProject(options?: HttpCommandOptions): Promise<WikiProject | null> {
  const res = await apiProjects(options)
  return res.currentProject ? toWikiProject(res.currentProject) : null
}

export async function getWebProjectByPath(path: string, options?: HttpCommandOptions): Promise<WikiProject | null> {
  const normalized = path.replace(/\\/g, "/").replace(/\/+$/, "")
  const res = await apiProjects(options)
  const project = res.projects.find((p) => p.path.replace(/\\/g, "/").replace(/\/+$/, "") === normalized) ?? null
  return project ? toWikiProject(project) : null
}
