import { describe, expect, it } from "vitest";
import {
  classifyFailure,
  summarizeBatch,
  type ImportOutcome,
} from "./importBatch";

const ok = (file: string): ImportOutcome => ({ kind: "imported", file });
const dupe = (file: string): ImportOutcome => ({ kind: "duplicate", file });
const bad = (file: string, error = "boom"): ImportOutcome => ({
  kind: "failed",
  file,
  error,
});

describe("classifyFailure", () => {
  it("treats the store's duplicate error as a skip, not a failure", () => {
    const e = "this demo is already imported (same file hash)";
    expect(classifyFailure("a.dem", e)).toEqual({
      kind: "duplicate",
      file: "a.dem",
    });
  });

  it("treats anything else as a real failure and keeps the message", () => {
    expect(classifyFailure("a.dem", "cannot read demo: no such file")).toEqual({
      kind: "failed",
      file: "a.dem",
      error: "cannot read demo: no such file",
    });
  });

  it("stringifies non-string rejections", () => {
    const out = classifyFailure("a.dem", new Error("parse blew up"));
    expect(out.kind).toBe("failed");
    expect(out).toHaveProperty("error", "Error: parse blew up");
  });
});

describe("summarizeBatch", () => {
  it("says nothing when nothing was picked", () => {
    expect(summarizeBatch([])).toBeNull();
  });

  it("says nothing after one clean import — the new row is the feedback", () => {
    expect(summarizeBatch([ok("a.dem")])).toBeNull();
  });

  it("names the file when a single import was already in the library", () => {
    expect(summarizeBatch([dupe("a.dem")])).toEqual({
      message: "a.dem is already in your library.",
      hadFailures: false,
    });
  });

  it("names the file and reason when a single import fails", () => {
    expect(summarizeBatch([bad("a.dem", "not a CS2 demo")])).toEqual({
      message: "a.dem failed: not a CS2 demo",
      hadFailures: true,
    });
  });

  it("counts a clean batch", () => {
    const s = summarizeBatch([ok("a.dem"), ok("b.dem"), ok("c.dem")]);
    expect(s).toEqual({
      message: "Imported 3 of 3 demos.",
      hadFailures: false,
    });
  });

  it("reports duplicates as skipped without flagging failure", () => {
    const s = summarizeBatch([ok("a.dem"), dupe("b.dem"), dupe("c.dem")]);
    expect(s?.message).toBe(
      "Imported 1 of 3 demos. 2 were already in your library.",
    );
    expect(s?.hadFailures).toBe(false);
  });

  it("uses singular wording for one duplicate", () => {
    const s = summarizeBatch([ok("a.dem"), dupe("b.dem")]);
    expect(s?.message).toBe(
      "Imported 1 of 2 demos. 1 was already in your library.",
    );
  });

  it("lists failures and flags the batch", () => {
    const s = summarizeBatch([ok("a.dem"), bad("b.dem", "corrupt header")]);
    expect(s?.message).toBe(
      "Imported 1 of 2 demos. b.dem failed: corrupt header",
    );
    expect(s?.hadFailures).toBe(true);
  });

  it("truncates a long failure list rather than dumping every error", () => {
    const s = summarizeBatch([
      ok("a.dem"),
      bad("b.dem"),
      bad("c.dem"),
      bad("d.dem"),
      bad("e.dem"),
    ]);
    expect(s?.message).toBe(
      "Imported 1 of 5 demos. b.dem failed: boom c.dem failed: boom 2 more failed.",
    );
    expect(s?.hadFailures).toBe(true);
  });

  it("handles a batch where everything failed", () => {
    const s = summarizeBatch([bad("a.dem", "x"), bad("b.dem", "y")]);
    expect(s?.message).toBe("a.dem failed: x b.dem failed: y");
    expect(s?.hadFailures).toBe(true);
  });

  it("handles a batch that was entirely duplicates", () => {
    const s = summarizeBatch([dupe("a.dem"), dupe("b.dem")]);
    expect(s).toEqual({
      message: "2 were already in your library.",
      hadFailures: false,
    });
  });
});
