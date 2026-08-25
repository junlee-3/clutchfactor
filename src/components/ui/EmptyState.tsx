import type { ReactNode } from "react";
import { Button } from "./Button";

interface EmptyStateAction {
  label: string;
  onClick: () => void;
}

interface EmptyStateProps {
  title: string;
  body: ReactNode;
  action?: EmptyStateAction;
  className?: string;
}

// "An invitation with a next action" (design-system.md §6): display-sans
// title, body copy, one optional primary action. Every screen's
// empty/zero-result state uses this — never a bare sentence (§9, §10
// no-layout-shift floor).
export function EmptyState({ title, body, action, className }: EmptyStateProps) {
  const classes = ["ui-empty", className].filter(Boolean).join(" ");
  return (
    <div className={classes}>
      <p className="ui-empty-title type-title">{title}</p>
      <p className="ui-empty-body type-body">{body}</p>
      {action && (
        <Button variant="primary" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
  );
}
