import type { ProgressEvent } from "../lib/ipc";

interface Props {
  fileName: string;
  progress: ProgressEvent | null;
}

export function ImportProgress({ fileName, progress }: Props) {
  const pct = Math.round((progress?.pct ?? 0) * 100);
  return (
    <div className="ui-import-row" role="status" aria-live="polite">
      <div className="ui-import-row-text">
        <span className="ui-import-file type-data">{fileName}</span>
        <span className="ui-import-detail type-data">
          {progress?.detail ?? "Starting import"} · {pct}%
        </span>
      </div>
      <div className="ui-progress-track">
        <div className="ui-progress-fill" style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
