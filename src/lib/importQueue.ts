// Multi-demo import queue state (issue #3). Pure — screens drive it, tests
// cover it, no Tauri imports here. Every file keeps its own status and
// error; a later failure never overwrites an earlier one (the M6-review
// last-error-only bug, fixed here for Library AND Corpus).
import { basename } from "./basename";

export type FileStatus = "pending" | "importing" | "done" | "skipped" | "failed";

export interface QueueFile {
  path: string;
  name: string;
  status: FileStatus;
  error?: string;
}

export function initQueue(paths: string[]): QueueFile[] {
  return paths.map((path) => ({ path, name: basename(path), status: "pending" }));
}

export function startFile(q: QueueFile[], i: number): QueueFile[] {
  return q.map((f, j) => (j === i ? { ...f, status: "importing" } : f));
}

/** Duplicate hash is a benign skip (re-picking a folder is normal), not a failure. */
export function isDuplicateError(message: string): boolean {
  return message.includes("already imported");
}

export function finishFile(q: QueueFile[], i: number, error?: string): QueueFile[] {
  return q.map((f, j) => {
    if (j !== i) return f;
    if (error === undefined) return { ...f, status: "done", error: undefined };
    if (isDuplicateError(error)) return { ...f, status: "skipped", error: undefined };
    return { ...f, status: "failed", error };
  });
}

export function queueDone(q: QueueFile[]): boolean {
  return q.every((f) => f.status === "done" || f.status === "skipped" || f.status === "failed");
}

/** §7 voice: what happened, then where to look. */
export function queueSummary(q: QueueFile[]): string {
  const done = q.filter((f) => f.status === "done").length;
  const skipped = q.filter((f) => f.status === "skipped").length;
  const failed = q.filter((f) => f.status === "failed").length;
  const parts: string[] = [];
  if (done > 0) parts.push(`${done} imported`);
  if (skipped > 0) parts.push(`${skipped} already in library`);
  if (failed > 0) parts.push(`${failed} failed — see below`);
  return parts.join(" · ");
}
