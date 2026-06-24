import { invoke } from "@tauri-apps/api/core"
import type { FileNode, WikiProject } from "@/types/wiki"
import { ensureProjectId, upsertProjectInfo } from "@/lib/project-identity"
import { isAbsolutePath } from "@/lib/path-utils"
import {
  apiHealth,
  apiProjectCreateDirectory,
  apiProjectDeleteFile,
  apiProjectFileContent,
  apiProjectFiles,
  apiProjectWriteFile,
  apiProjectWriteFileAtomic,
  apiProjects,
} from "@/commands/http-api"
import { toFileNode, toWikiProject } from "@/commands/web-equivalent"
import { isWebMode } from "@/lib/web-mode"

/** Raw shape returned by the Rust commands — id is attached client-side. */
interface RawProject {
  name: string
  path: string
}

function assertWebUnsupported(operation: string): void {
  if (isWebMode()) {
    throw new Error(`${operation} is not available in Web mode until the equivalent HTTP API is implemented`)
  }
}

function normalizeFsPath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "")
}

async function resolveWebProjectForPath(path: string): Promise<{ id: string; name: string; path: string; current: boolean; relativePath: string }> {
  const res = await apiProjects()
  const normalized = normalizeFsPath(path)
  const projects = res.projects
  const project = projects.find((p) => normalizeFsPath(p.path) === normalized)
    ?? projects.find((p) => normalized.startsWith(`${normalizeFsPath(p.path)}/`))
    ?? res.currentProject

  if (!project) {
    throw new Error(`No Web API project matches path: ${path}`)
  }

  const root = normalizeFsPath(project.path)
  const relativePath = normalized === root
    ? ""
    : normalized.startsWith(`${root}/`)
      ? normalized.slice(root.length + 1)
      : normalized

  return { ...project, relativePath }
}

function rootForRelativePath(relativePath: string): "wiki" | "sources" | "all" {
  const rel = normalizeFsPath(relativePath).replace(/^\/+/, "")
  if (rel === "wiki" || rel.startsWith("wiki/")) return "wiki"
  if (rel === "raw/sources" || rel.startsWith("raw/sources/")) return "sources"
  return "all"
}

export async function readFile(
  path: string,
  options?: { extractImages?: boolean },
): Promise<string> {
  if (isWebMode()) {
    const project = await resolveWebProjectForPath(path)
    const relativePath = project.relativePath || path
    const res = await apiProjectFileContent(project.id, relativePath)
    return res.content
  }
  return invoke<string>("read_file", {
    path,
    extractImages: options?.extractImages,
  })
}

export async function writeFile(path: string, contents: string): Promise<void> {
  if (isWebMode()) {
    const project = await resolveWebProjectForPath(path)
    await apiProjectWriteFile(project.id, path, contents)
    return
  }
  assertAbsoluteFsPath("writeFile", path)
  return invoke<void>("write_file", { path, contents })
}

export async function writeFileBase64(path: string, base64: string): Promise<void> {
  assertWebUnsupported("writeFileBase64")
  assertAbsoluteFsPath("writeFileBase64", path)
  return invoke<void>("write_file_base64", { path, base64 })
}

export async function writeFileAtomic(path: string, contents: string): Promise<void> {
  if (isWebMode()) {
    const project = await resolveWebProjectForPath(path)
    await apiProjectWriteFileAtomic(project.id, path, contents)
    return
  }
  assertAbsoluteFsPath("writeFileAtomic", path)
  return invoke<void>("write_file_atomic", { path, contents })
}

export async function listDirectory(path: string): Promise<FileNode[]> {
  if (isWebMode()) {
    const project = await resolveWebProjectForPath(path)
    const res = await apiProjectFiles(project.id, {
      root: rootForRelativePath(project.relativePath),
      recursive: true,
    })
    return res.files.map(toFileNode)
  }
  return invoke<FileNode[]>("list_directory", { path })
}

export async function copyFile(
  source: string,
  destination: string
): Promise<void> {
  assertWebUnsupported("copyFile")
  return invoke("copy_file", { source, destination })
}

export async function copyDirectory(
  source: string,
  destination: string
): Promise<string[]> {
  assertWebUnsupported("copyDirectory")
  return invoke<string[]>("copy_directory", { source, destination })
}

export async function preprocessFile(path: string): Promise<string> {
  assertWebUnsupported("preprocessFile")
  return invoke<string>("preprocess_file", { path })
}

export async function deleteFile(path: string): Promise<void> {
  if (isWebMode()) {
    const project = await resolveWebProjectForPath(path)
    await apiProjectDeleteFile(project.id, path)
    return
  }
  return invoke("delete_file", { path })
}

export async function findRelatedWikiPages(
  projectPath: string,
  sourceName: string
): Promise<string[]> {
  return invoke<string[]>("find_related_wiki_pages", { projectPath, sourceName })
}

export async function createDirectory(path: string): Promise<void> {
  if (isWebMode()) {
    const project = await resolveWebProjectForPath(path)
    await apiProjectCreateDirectory(project.id, path)
    return
  }
  assertAbsoluteFsPath("createDirectory", path)
  return invoke<void>("create_directory", { path })
}

export async function fileExists(path: string): Promise<boolean> {
  if (isWebMode()) {
    try {
      await readFile(path)
      return true
    } catch {
      return false
    }
  }
  return invoke<boolean>("file_exists", { path })
}

export async function getFileModifiedTime(path: string): Promise<number> {
  return invoke<number>("get_file_modified_time", { path })
}

export async function getFileSize(path: string): Promise<number> {
  return invoke<number>("get_file_size", { path })
}

export async function getFileMd5(path: string): Promise<string> {
  return invoke<string>("get_file_md5", { path })
}

function assertAbsoluteFsPath(operation: string, path: string): void {
  if (!isAbsolutePath(path)) {
    throw new Error(`${operation} requires an absolute path: ${path}`)
  }
}

/** Mirror of `commands::fs::FileBase64` (Rust side). */
export interface FileBase64 {
  base64: string
  mimeType: string
}

/**
 * Read any file off disk as base64 + a guessed mime type. The
 * vision-caption pipeline uses this to pick up extracted images
 * without having to read them as UTF-8 strings (PNG bytes aren't
 * valid UTF-8 — `readFile` would corrupt them).
 */
export async function readFileAsBase64(path: string): Promise<FileBase64> {
  assertWebUnsupported("readFileAsBase64")
  return invoke<FileBase64>("read_file_as_base64", { path })
}

export async function createProject(
  name: string,
  path: string,
): Promise<WikiProject> {
  assertWebUnsupported("createProject")
  const raw = await invoke<RawProject>("create_project", { name, path })
  const id = await ensureProjectId(raw.path)
  await upsertProjectInfo(id, raw.path, raw.name)
  return { id, name: raw.name, path: raw.path }
}

export async function openProject(path: string): Promise<WikiProject> {
  if (isWebMode()) {
    const res = await apiProjects()
    const normalized = normalizeFsPath(path)
    const project = res.projects.find((p) => p.id === path || normalizeFsPath(p.path) === normalized)
      ?? res.currentProject
    if (!project) {
      throw new Error(`No Web API project matches path: ${path}`)
    }
    return toWikiProject(project)
  }
  const raw = await invoke<RawProject>("open_project", { path })
  const id = await ensureProjectId(raw.path)
  await upsertProjectInfo(id, raw.path, raw.name)
  return { id, name: raw.name, path: raw.path }
}

export async function openProjectFolder(path: string): Promise<void> {
  assertWebUnsupported("openProjectFolder")
  return invoke<void>("open_project_folder", { path })
}

export async function clipServerStatus(): Promise<string> {
  return invoke<string>("clip_server_status")
}

export async function apiServerStatus(): Promise<string> {
  if (isWebMode()) {
    const health = await apiHealth()
    return health.status
  }
  return invoke<string>("api_server_status")
}

export async function apiServerReloadConfig(): Promise<string> {
  assertWebUnsupported("apiServerReloadConfig")
  return invoke<string>("api_server_reload_config")
}

export async function mcpServerEntryPath(): Promise<string> {
  return invoke<string>("mcp_server_entry_path")
}
