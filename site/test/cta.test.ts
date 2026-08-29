// @vitest-environment happy-dom
import { beforeEach, describe, expect, it } from "vitest";
import { applyNavDownload, renderDownloadButtons } from "../src/cta";
import { assetUrl, release } from "../src/release";

beforeEach(() => {
  document.body.innerHTML = `
    <a data-nav-download href="#download">Download v1.0.0</a>
    <div data-download-buttons>
      <a class="btn btn--primary" data-os="windows" href="#">Download for Windows <small>x</small></a>
      <a class="btn btn--secondary" data-os="mac" href="#">Download for macOS <small>x</small></a>
    </div>`;
});

const btn = (os: string) => document.querySelector<HTMLAnchorElement>(`[data-os="${os}"]`)!;

describe("renderDownloadButtons", () => {
  it("makes the visitor's OS the primary button with the real asset and size", () => {
    renderDownloadButtons(document, "mac");
    expect(btn("mac").classList.contains("btn--primary")).toBe(true);
    expect(btn("windows").classList.contains("btn--secondary")).toBe(true);
    expect(btn("mac").href).toBe(assetUrl(release.mac.file));
    expect(btn("mac").querySelector("small")!.textContent).toBe(".dmg · Apple silicon · 10 MB");
    expect(btn("windows").querySelector("small")!.textContent).toBe(".exe · 8 MB");
  });

  it("shows both as secondary on an unknown OS", () => {
    renderDownloadButtons(document, "other");
    expect(btn("mac").classList.contains("btn--secondary")).toBe(true);
    expect(btn("windows").classList.contains("btn--secondary")).toBe(true);
    expect(btn("mac").classList.contains("btn--primary")).toBe(false);
  });

  it("applies to every [data-download-buttons] container", () => {
    document.body.innerHTML += `<div class="dl" data-download-buttons><a class="btn btn--secondary" data-os="mac" href="#">m <small>x</small></a><a class="btn btn--primary" data-os="windows" href="#">w <small>x</small></a></div>`;
    renderDownloadButtons(document, "mac");
    const macs = document.querySelectorAll<HTMLAnchorElement>('[data-os="mac"]');
    expect(macs.length).toBe(2);
    for (const a of macs) {
      expect(a.classList.contains("btn--primary")).toBe(true);
      expect(a.href).toBe(assetUrl(release.mac.file));
    }
    for (const a of document.querySelectorAll<HTMLAnchorElement>('[data-os="windows"]')) {
      expect(a.classList.contains("btn--secondary")).toBe(true);
    }
  });
});

describe("applyNavDownload", () => {
  it("points the nav button at the visitor's installer", () => {
    applyNavDownload(document, "windows");
    expect(document.querySelector<HTMLAnchorElement>("[data-nav-download]")!.href).toBe(assetUrl(release.win.file));
  });
  it("keeps #download for unknown OS", () => {
    applyNavDownload(document, "other");
    expect(document.querySelector<HTMLAnchorElement>("[data-nav-download]")!.getAttribute("href")).toBe("#download");
  });
});
