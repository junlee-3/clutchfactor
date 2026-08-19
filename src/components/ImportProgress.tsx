import type { ProgressEvent } from "../lib/ipc";

interface Props {
  fileName: string;
  progress: ProgressEvent | null;
}

export function ImportProgress({ fileName, progress }: Props) {
  const pct = Math.round((progress?.pct ?? 0) * 100);
  return (
    <div className="import-row" role="status" aria-live="polite">
      <div className="import-row-text">
        <span className="import-file">{fileName}</span>
        <span className="import-detail">
          {progress?.detail ?? "Starting import"} · {pct}%
        </span>
      </div>
      <div className="progress-track">
        <div className="progress-fill" style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
