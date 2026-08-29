import type { Platform } from "./platform";
import { assetUrl, formatMb, release } from "./release";

const label = {
  mac: () => `.dmg · ${release.mac.arch} · ${formatMb(release.mac.bytes)}`,
  windows: () => `.exe · ${formatMb(release.win.bytes)}`,
};
const file = { mac: release.mac.file, windows: release.win.file };

/** Primary = the visitor's OS; the other OS is secondary; unknown OS → both secondary.
 *  Applies to every `[data-download-buttons]` container in the document (the hero's
 *  `.cta` and the download section's `.dl`), not just the first. */
export function renderDownloadButtons(doc: Document, platform: Platform): void {
  for (const os of ["mac", "windows"] as const) {
    const anchors = doc.querySelectorAll<HTMLAnchorElement>(`[data-download-buttons] a[data-os="${os}"]`);
    for (const a of anchors) {
      a.href = assetUrl(file[os]);
      const small = a.querySelector("small");
      if (small) small.textContent = label[os]();
      const primary = platform === os;
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
