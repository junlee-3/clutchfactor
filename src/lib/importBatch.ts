// Batch demo import: outcome classification and the one-line banner the
// Library shows afterwards. Pure so the counting and wording are unit-tested;
// the screen only awaits the imports and collects results.

export type ImportOutcome =
  | { kind: "imported"; file: string }
  | { kind: "duplicate"; file: string }
  | { kind: "failed"; file: string; error: string };

// cf-store's StoreError::DuplicateImport text (crates/cf-store/src/store.rs).
// Hand-mirrored like the types in ipc.ts — update both sides together.
// A duplicate is a *skip*, not a failure: re-picking a folder you've already
// imported is normal, and it costs only a hash because save_match rejects on
// file_hash before the parse runs.
const DUPLICATE_MARKER = "already imported";

// A one-line banner can't carry a dozen error strings; the rest are counted.
const MAX_LISTED_FAILURES = 2;

export interface BatchSummary {
  message: string;
  hadFailures: boolean;
}

/// Sort a rejected import into "already had it" vs "actually broke".
export function classifyFailure(file: string, error: unknown): ImportOutcome {
  const message = String(error);
  return message.includes(DUPLICATE_MARKER)
    ? { kind: "duplicate", file }
    : { kind: "failed", file, error: message };
}

/// Null means "say nothing" — a clean single import needs no banner, the new
/// library row is the feedback.
export function summarizeBatch(outcomes: ImportOutcome[]): BatchSummary | null {
  if (outcomes.length === 0) return null;

  if (outcomes.length === 1) {
    const [only] = outcomes;
    switch (only.kind) {
      case "imported":
        return null;
      case "duplicate":
        return {
          message: `${only.file} is already in your library.`,
          hadFailures: false,
        };
      case "failed":
        return {
          message: `${only.file} failed: ${only.error}`,
          hadFailures: true,
        };
    }
  }

  const imported = outcomes.filter((o) => o.kind === "imported").length;
  const duplicates = outcomes.filter((o) => o.kind === "duplicate").length;
  const failures = outcomes.filter(
    (o): o is Extract<ImportOutcome, { kind: "failed" }> => o.kind === "failed",
  );

  const parts: string[] = [];
  if (imported > 0) {
    parts.push(`Imported ${imported} of ${outcomes.length} demos.`);
  }
  if (duplicates > 0) {
    parts.push(
      duplicates === 1
        ? "1 was already in your library."
        : `${duplicates} were already in your library.`,
    );
  }
  for (const f of failures.slice(0, MAX_LISTED_FAILURES)) {
    parts.push(`${f.file} failed: ${f.error}`);
  }
  const unlisted = failures.length - MAX_LISTED_FAILURES;
  if (unlisted > 0) {
    parts.push(`${unlisted} more failed.`);
  }

  return { message: parts.join(" "), hadFailures: failures.length > 0 };
}
