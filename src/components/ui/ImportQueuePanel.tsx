import type { ProgressEvent } from "../../lib/ipc";
import { queueDone, queueSummary, type FileStatus, type QueueFile } from "../../lib/importQueue";
import { ImportProgress } from "../ImportProgress";
import { Button } from "./Button";
import { cardClass } from "./classes";

interface ImportQueuePanelProps {
  queue: QueueFile[];
  progress: ProgressEvent | null;
  current: QueueFile | null;
  onDismiss: () => void;
}

const STATUS_LABEL: Record<FileStatus, (f: QueueFile) => string> = {
  pending: () => "waiting",
  importing: () => "importing…",
  done: () => "imported",
  skipped: () => "already in library",
  failed: (f) => f.error ?? "failed",
};

// Extracted from the duplicated Library/Corpus import-queue block
// (design-system.md §6, §9). Consumes the existing pure importQueue.ts
// module (unchanged) and ImportProgress; screens keep owning the queue
// STATE (Tasks 6/10 wire this in) — this component only renders it.
export function ImportQueuePanel({ queue, progress, current, onDismiss }: ImportQueuePanelProps) {
  const done = queueDone(queue);
  const failed = queue.some((f) => f.status === "failed");

  return (
    <div className="ui-queue">
      {current && (
        <ImportProgress
          fileName={`${queue.indexOf(current) + 1} of ${queue.length}: ${current.name}`}
          progress={progress}
        />
      )}
      <ul className="ui-queue-list">
        {queue.map((f) => (
          <li key={f.path} className={`ui-queue-row ui-queue-row-${f.status}`}>
            <span className="ui-queue-file type-data">{f.name}</span>
            <span className="ui-queue-detail type-data">{STATUS_LABEL[f.status](f)}</span>
          </li>
        ))}
      </ul>
      {done && (
        <div
          className={`${cardClass(failed ? "loss" : undefined)} ui-queue-summary`}
          role={failed ? "alert" : "status"}
        >
          <span className="type-body">{queueSummary(queue)}</span>
          <Button variant="secondary" size="sm" onClick={onDismiss}>
            Dismiss
          </Button>
        </div>
      )}
    </div>
  );
}
