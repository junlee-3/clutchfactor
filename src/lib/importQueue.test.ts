import { describe, expect, it } from "vitest";
import {
  finishFile,
  initQueue,
  isDuplicateError,
  queueDone,
  queueSummary,
  startFile,
} from "./importQueue";

describe("importQueue", () => {
  const paths = ["/demos/a.dem", "C:\\demos\\b.dem", "/demos/c.dem"];

  it("initializes pending files with display names", () => {
    const q = initQueue(paths);
    expect(q.map((f) => f.name)).toEqual(["a.dem", "b.dem", "c.dem"]);
    expect(q.every((f) => f.status === "pending")).toBe(true);
  });

  it("keeps every file's error — never only the last", () => {
    let q = initQueue(paths);
    q = finishFile(startFile(q, 0), 0, "Couldn't parse a.dem: bad header.");
    q = finishFile(startFile(q, 1), 1);
    q = finishFile(startFile(q, 2), 2, "Couldn't parse c.dem: truncated.");
    expect(q[0]).toMatchObject({ status: "failed", error: "Couldn't parse a.dem: bad header." });
    expect(q[1].status).toBe("done");
    expect(q[2]).toMatchObject({ status: "failed", error: "Couldn't parse c.dem: truncated." });
  });

  it("classifies duplicate-hash errors as benign skips", () => {
    expect(isDuplicateError("this demo is already imported (same file hash)")).toBe(true);
    expect(isDuplicateError("Couldn't parse x.dem: nope")).toBe(false);
    let q = initQueue(["/demos/a.dem"]);
    q = finishFile(startFile(q, 0), 0, "this demo is already imported (same file hash)");
    expect(q[0].status).toBe("skipped");
    expect(q[0].error).toBeUndefined();
  });

  it("does not mutate its input", () => {
    const q = initQueue(paths);
    startFile(q, 0);
    expect(q[0].status).toBe("pending");
  });

  it("summarizes in §7 voice", () => {
    let q = initQueue(paths);
    q = finishFile(startFile(q, 0), 0);
    q = finishFile(startFile(q, 1), 1, "this demo is already imported (same file hash)");
    q = finishFile(startFile(q, 2), 2, "Couldn't parse c.dem: truncated.");
    expect(queueDone(q)).toBe(true);
    expect(queueSummary(q)).toBe("1 imported · 1 already in library · 1 failed — see below");
  });

  it("summary omits empty categories", () => {
    let q = initQueue(["/demos/a.dem"]);
    q = finishFile(startFile(q, 0), 0);
    expect(queueSummary(q)).toBe("1 imported");
  });
});
