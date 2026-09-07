/** The ONE place the site knows about a release. Update on every tag
 *  (CLAUDE.md release checklist): version, tag, file names, byte sizes
 *  (`gh release view vX.Y.Z --json assets`). */
export const release = {
  version: "1.0.0",
  tag: "v1.0.0",
  mac: { file: "ClutchFactor_1.0.0_aarch64.dmg", bytes: 10549192, arch: "Apple silicon" },
  win: { file: "ClutchFactor_1.0.0_x64-setup.exe", bytes: 7936273 },
  msi: { file: "ClutchFactor_1.0.0_x64_en-US.msi", bytes: 10166272 },
} as const;

export const REPO_URL = "https://github.com/junlee-3/clutchfactor";

export const assetUrl = (file: string): string =>
  `${REPO_URL}/releases/download/${release.tag}/${file}`;

/** Whole MiB, the number Finder/Explorer show for these files. */
export const formatMb = (bytes: number): string => `${Math.round(bytes / 1048576)} MB`;
