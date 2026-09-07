import { describe, expect, it } from "vitest";
import { REPO_URL, assetUrl, formatMb, release } from "../src/release";

describe("release constants", () => {
  it("points at the v1.0.0 GitHub release assets", () => {
    expect(assetUrl(release.mac.file)).toBe(
      "https://github.com/junlee-3/clutchfactor/releases/download/v1.0.0/ClutchFactor_1.0.0_aarch64.dmg",
    );
    expect(assetUrl(release.win.file)).toBe(`${REPO_URL}/releases/download/v1.0.0/ClutchFactor_1.0.0_x64-setup.exe`);
    expect(assetUrl(release.msi.file)).toBe(`${REPO_URL}/releases/download/v1.0.0/ClutchFactor_1.0.0_x64_en-US.msi`);
  });

  it("renders whole MiB the way the download page quotes them", () => {
    expect(formatMb(release.mac.bytes)).toBe("10 MB");
    expect(formatMb(release.win.bytes)).toBe("8 MB");
    expect(formatMb(release.msi.bytes)).toBe("10 MB");
  });
});
