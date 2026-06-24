import { beforeEach, describe, expect, it, vi } from "vitest"
import { healthApiPath, httpGet, httpPost, projectApiPath } from "@/lib/http-command-client"

describe("http-command-client", () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  it("builds stable API paths", () => {
    expect(healthApiPath()).toBe("/api/v1/health")
    expect(projectApiPath("current", "/graph")).toBe("/api/v1/projects/current/graph")
  })

  it("parses successful JSON responses", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true, text: async () => '{"ok":true,"value":1}' }))
    await expect(httpGet<{ value: number }>("/api/v1/health")).resolves.toMatchObject({ value: 1 })
  })

  it("throws API errors", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 500, statusText: "boom", text: async () => '{"ok":false,"error":"bad"}' }))
    await expect(httpPost("/api/v1/test", {})).rejects.toThrow("bad")
  })
})
