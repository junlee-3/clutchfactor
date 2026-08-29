import type { Platform } from "./platform";
import { assetUrl, formatMb, release } from "./release";

const label = {
  mac: { full: () => `.dmg · ${release.mac.arch} · ${formatMb(release.mac.bytes)}`, short: () => `${release.mac.arch} · ${formatMb(release.mac.bytes)}` },
  windows: { full: () => `.exe · ${formatMb(release.win.bytes)}`, short: () => formatMb(release.win.bytes) },
};
const file = { mac: release.mac.file, windows: release.win.file };

/** Primary = the visitor's OS. Unknown OS (iOS, Android, Linux) falls back to
 *  Windows primary / macOS secondary — the same as the static HTML, so every
 *  screen keeps exactly one accent button (spec §3.2). */
const primaryFor = (platform: Platform): "mac" | "windows" => (platform === "mac" ? "mac" : "windows");

/** Applies to every `[data-download-buttons]` container in the document (the hero's
 *  `.cta` and the download section's `.dl`), not just the first. An anchor marked
 *  `data-label="short"` drops the extension its own text already shows. */
export function renderDownloadButtons(doc: Document, platform: Platform): void {
  for (const os of ["mac", "windows"] as const) {
    const anchors = doc.querySelectorAll<HTMLAnchorElement>(`[data-download-buttons] a[data-os="${os}"]`);
    for (const a of anchors) {
      a.href = assetUrl(file[os]);
      const small = a.querySelector("small");
      if (small) small.textContent = a.dataset.label === "short" ? label[os].short() : label[os].full();
      const primary = primaryFor(platform) === os;
      a.classList.toggle("btn--primary", primary);
      a.classList.toggle("btn--secondary", !primary);
    }
  }
}

export function applyNavDownload(doc: Document, platform: Platform): void {
  const a = doc.querySelector<HTMLAnchorElement>("[data-nav-download]");
  if (!a) return;
  if (platform === "mac" || platform === "windows") a.href = assetUrl(file[platform]);
}
