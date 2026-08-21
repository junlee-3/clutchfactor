import type { ButtonHTMLAttributes, ReactNode } from "react";
import { buttonClass, type ButtonSize, type ButtonVariant } from "./classes";

interface ButtonProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "type"> {
  variant: ButtonVariant;
  size?: ButtonSize;
  className?: string;
  children?: ReactNode;
}

// The app's one button. Screens compose this, never re-declare a button
// surface (design-system.md §6). Danger's two-step confirm pattern is a
// screen-level concern — pair variant="danger" with the "ui-btn-armed"
// modifier class (components.css) for the second, confirming step.
export function Button({
  variant,
  size = "md",
  className,
  children,
  ...rest
}: ButtonProps) {
  const classes = [buttonClass(variant, size), className].filter(Boolean).join(" ");
  return (
    <button type="button" className={classes} {...rest}>
      {children}
    </button>
  );
}
