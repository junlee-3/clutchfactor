import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { addToast, expire, type ToastItem, type ToastKind } from "../../lib/toast";

interface ToastContextValue {
  push: (kind: ToastKind, text: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

/** How often we sweep for expired toasts. Coarser than the 5s TTL itself —
 * only cosmetic (how promptly a stale toast disappears), never the source
 * of truth (that's toast.ts's `expire`, driven by real Date.now()). */
const EXPIRE_POLL_MS = 250;

// Bottom-right toast stack (design-system.md §6): --shadow-float, auto-
// dismiss, role="status"/"alert" per kind. State lives here; the queue math
// (cap at 3, 5s TTL) is the pure src/lib/toast.ts module Date.now() feeds.
export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const push = useCallback((kind: ToastKind, text: string) => {
    setToasts((list) => addToast(list, kind, text, Date.now()));
  }, []);

  // Only poll while there's something to expire — no timer running on a
  // quiet screen.
  useEffect(() => {
    if (toasts.length === 0) return;
    const id = window.setInterval(() => {
      setToasts((list) => expire(list, Date.now()));
    }, EXPIRE_POLL_MS);
    return () => window.clearInterval(id);
  }, [toasts.length]);

  return (
    <ToastContext.Provider value={{ push }}>
      {children}
      <div className="ui-toast-container">
        {toasts.map((t) => (
          <div
            key={t.id}
            className={`ui-toast ui-toast-${t.kind}`}
            role={t.kind === "error" ? "alert" : "status"}
          >
            {t.text}
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within a <ToastProvider>");
  return ctx;
}
