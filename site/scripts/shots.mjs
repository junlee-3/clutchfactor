// Screenshots → WebP at two widths, the radar fallback, and og.jpg.
// Run from anywhere: `pnpm -C site shots`. Outputs are committed.
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..", "..");            // repo root
const out = path.resolve(here, "..", "public", "shots");
const pub = path.resolve(here, "..", "public");

const shots = [
  ["docs/screenshots/report.png", "report"],
  ["docs/screenshots/replay.png", "replay"],
  ["docs/screenshots/trends.png", "trends"],
  ["docs/screenshots/corpus.png", "corpus"],
  ["docs/screenshots/library.png", "library"],
  ["docs/design/walkthrough-v1.3/report-coach.png", "coach"],
  ["docs/design/walkthrough-v1.4/04-watches.png", "watches"],
];

await mkdir(out, { recursive: true });

for (const [src, name] of shots) {
  for (const width of [1440, 960]) {
    const file = path.join(out, `${name}-${width}.webp`);
    await sharp(path.join(root, src)).resize({ width }).webp({ quality: 82 }).toFile(file);
    console.log("wrote", path.relative(root, file));
  }
}

await sharp(path.join(root, "assets/maps/de_inferno.png"))
  .resize({ width: 1600 })
  .webp({ quality: 70 })
  .toFile(path.join(out, "radar-inferno.webp"));
console.log("wrote site/public/shots/radar-inferno.webp");

await sharp(path.join(root, "docs/screenshots/report.png"))
  .resize(1200, 630, { fit: "cover", position: "top" })
  .jpeg({ quality: 82 })
  .toFile(path.join(pub, "og.jpg"));
console.log("wrote site/public/og.jpg");
