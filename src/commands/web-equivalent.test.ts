import { describe, expect, it } from "vitest"
import { toFileNode, toWikiProject } from "@/commands/web-equivalent"

describe("web-equivalent facade", () => {
  it("maps project type", () => {
    expect(toWikiProject({ id: "p1", name: "Demo", path: "/tmp/demo" })).toEqual({ id: "p1", name: "Demo", path: "/tmp/demo" })
  })

  it("maps file node type", () => {
    expect(toFileNode({ name: "index.md", path: "wiki/index.md", isDir: false })).toEqual({ name: "index.md", path: "wiki/index.md", is_dir: false, children: undefined })
  })
})
