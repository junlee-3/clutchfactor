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
  it("forces a rejection for the command named by VITE_FAIL_IPC, in dev, without calling invoke", async () => {
    vi.stubEnv("VITE_FAIL_IPC", "list_matches");
    try {
      const { call } = await import("./ipc");
      await expect(call("list_matches")).rejects.toThrow(/forced failure/);
      expect(invoke).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });
});

describe("callRaw", () => {
  beforeEach(() => invoke.mockClear());
  it("sends the bytes as the request body and the metadata as headers", async () => {
    const { callRaw } = await import("./ipc");
    const bytes = new Uint8Array([1, 2, 3]);
    await expect(
      callRaw<number>("save_clip", bytes, { "x-clip-name": "mirage-r1-0m00s.mp4" }),
    ).resolves.toBe(42);
    expect(invoke).toHaveBeenCalledWith("save_clip", bytes, {
      headers: { "x-clip-name": "mirage-r1-0m00s.mp4" },
    });
  });
  it("records a performance measure named after the command in dev", async () => {
    const { callRaw } = await import("./ipc");
    const measure = vi.spyOn(performance, "measure");
    await callRaw("save_clip", new Uint8Array(), {});
    expect(measure.mock.calls.some((c) => c[0] === "ipc:save_clip")).toBe(true);
  });
  it("forces a rejection for the command named by VITE_FAIL_IPC, in dev, without calling invoke", async () => {
    vi.stubEnv("VITE_FAIL_IPC", "save_clip");
    try {
      const { callRaw } = await import("./ipc");
      await expect(callRaw("save_clip", new Uint8Array(), {})).rejects.toThrow(
        /forced failure/,
      );
      expect(invoke).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });
});
