import { getWebProjectGraph, getCurrentWebProject } from "@/commands/web-equivalent"
import { normalizePath } from "@/lib/path-utils"
import type { GraphEdge, GraphNode, CommunityInfo } from "@/lib/wiki-graph"

interface ApiGraphNodeLike {
  id?: unknown
  label?: unknown
  type?: unknown
  nodeType?: unknown
  path?: unknown
  linkCount?: unknown
  link_count?: unknown
  community?: unknown
}

interface ApiGraphEdgeLike {
  source?: unknown
  target?: unknown
  weight?: unknown
}

function asString(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback
}

function asNumber(value: unknown, fallback = 0): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback
}

function toAbsoluteProjectPath(projectPath: string, path: string): string {
  const pp = normalizePath(projectPath)
  const normalized = normalizePath(path).replace(/^\/+/, "")
  return normalized === pp || normalized.startsWith(`${pp}/`)
    ? normalized
    : `${pp}/${normalized}`
}

function buildSingleCommunity(nodes: GraphNode[]): CommunityInfo[] {
  if (nodes.length === 0) return []
  return [{
    id: 0,
    nodeCount: nodes.length,
    cohesion: 0,
    topNodes: nodes
      .slice()
      .sort((a, b) => b.linkCount - a.linkCount)
      .slice(0, 5)
      .map((node) => node.label),
  }]
}

export async function loadWebGraphData(projectId = "current"): Promise<{ nodes: GraphNode[]; edges: GraphEdge[]; communities: CommunityInfo[] }> {
  const [response, project] = await Promise.all([
    getWebProjectGraph(projectId),
    getCurrentWebProject(),
  ])
  const projectPath = project?.path ?? ""
  const nodes = (response.nodes as ApiGraphNodeLike[]).map((node) => {
    const rawPath = asString(node.path)
    return {
      id: asString(node.id),
      label: asString(node.label, asString(node.id)),
      type: asString(node.type ?? node.nodeType, "other"),
      path: projectPath ? toAbsoluteProjectPath(projectPath, rawPath) : rawPath,
      linkCount: asNumber(node.linkCount ?? node.link_count, 0),
      community: asNumber(node.community, 0),
    }
  }).filter((node) => node.id && node.path)

  const nodeIds = new Set(nodes.map((node) => node.id))
  const edges = (response.edges as ApiGraphEdgeLike[]).map((edge) => ({
    source: asString(edge.source),
    target: asString(edge.target),
    weight: asNumber(edge.weight, 1),
  })).filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target))

  return {
    nodes,
    edges,
    communities: buildSingleCommunity(nodes),
  }
}
