/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Dev-only kill switch (polish-and-release.md §2): when set to a command
   *  name, `call()` forces that command to reject so its error state can be
   *  provoked without touching the Rust side. Never read in production. */
  readonly VITE_FAIL_IPC?: string;
}
