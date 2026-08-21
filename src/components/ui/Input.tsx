import type { InputHTMLAttributes } from "react";

interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "className"> {
  /** Switches to the mono/data face — for numeric-ish or filename inputs. */
  mono?: boolean;
  className?: string;
}

// Thin styled input (design-system.md §6): bg2, line border, r-sm, chalk
// caret. Focus ring comes from the app's one :focus-visible treatment
// (base.css) — nothing button/input-specific needed here.
export function Input({ mono, className, ...rest }: InputProps) {
  const classes = ["ui-input", mono ? "ui-input-mono" : null, className]
    .filter(Boolean)
    .join(" ");
  return <input className={classes} {...rest} />;
}
