import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(async () => 42),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke, Channel: class {} }));

describe("call", () => {
  beforeEach(() => invoke.mockClear());
  it("forwards the command and args to invoke and returns its result", async () => {
    const { call } = await import("./ipc");
    await expect(call<number>("get_thing", { id: 7 })).resolves.toBe(42);
    expect(invoke).toHaveBeenCalledWith("get_thing", { id: 7 });
  });
  it("records a performance measure named after the command in dev", async () => {
    const { call } = await import("./ipc");
    const measure = vi.spyOn(performance, "measure");
    await call("list_matches");
    expect(measure.mock.calls.some((c) => c[0] === "ipc:list_matches")).toBe(true);
  });
});
