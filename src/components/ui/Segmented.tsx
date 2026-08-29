import { useRef } from "react";
import { segTabIndex } from "./classes";

interface SegmentedOption {
  value: string;
  label: string;
}

interface SegmentedProps {
  options: SegmentedOption[];
  value: string;
  onChange: (value: string) => void;
  ariaLabel: string;
  /** Locks the whole group (the replay disables its speed control while a
   *  clip is recording — the clip is a 1x record of what was played). */
  disabled?: boolean;
}

// Segmented control / radiogroup (design-system.md §6) — replaces the
// speed/side/phase/map chip one-offs with one component. Mirrors the
// role="radiogroup" + role="radio" + aria-checked semantics already used by
// Corpus.tsx's side/phase chips, and adds what those didn't have: real
// roving-tabindex keyboard support, so the group is a single tab stop and
// Left/Right arrows move both focus and selection (WAI-ARIA radiogroup
// pattern).
export function Segmented({
  options,
  value,
  onChange,
  ariaLabel,
  disabled,
}: SegmentedProps) {
  const optionRefs = useRef<Array<HTMLButtonElement | null>>([]);

  function handleKeyDown(event: React.KeyboardEvent<HTMLButtonElement>, index: number) {
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
    event.preventDefault();
    if (options.length === 0) return;
    const dir = event.key === "ArrowRight" ? 1 : -1;
    const next = (index + dir + options.length) % options.length;
    onChange(options[next].value);
    optionRefs.current[next]?.focus();
  }

  // -1 when `value` matches no option (stale filter value, async default,
  // call-site typo) — segTabIndex then makes the first option tabbable so
  // the group is never unreachable by Tab (WAI-ARIA APG radiogroup pattern).
  const activeIndex = options.findIndex((option) => option.value === value);

  return (
    <div className="ui-seg" role="radiogroup" aria-label={ariaLabel}>
      {options.map((option, i) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            ref={(el) => {
              optionRefs.current[i] = el;
            }}
            type="button"
            role="radio"
            aria-checked={active}
            disabled={disabled}
            tabIndex={segTabIndex(i, activeIndex)}
            className={`ui-seg-option${active ? " ui-seg-option-active" : ""}`}
            onClick={() => onChange(option.value)}
            onKeyDown={(e) => handleKeyDown(e, i)}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
