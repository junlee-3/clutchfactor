import "./styles/tokens.css";
import "./styles/site.css";

import { parseClipsManifest, shouldPlayVideo } from "./clips";
import { applyNavDownload, renderDownloadButtons } from "./cta";
import { initHero } from "./hero";
import { ledgerSchedule } from "./ledger";
import { detectPlatform } from "./platform";

document.documentElement.classList.add("js");

const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

// Download buttons follow the visitor's OS.
const platform = detectPlatform(navigator.userAgent, navigator.maxTouchPoints);
renderDownloadButtons(document, platform);
applyNavDownload(document, platform);

// Nav turns solid once the hero starts scrolling away.
const nav = document.querySelector<HTMLElement>("[data-nav]");
const onScroll = () => nav?.classList.toggle("nav--solid", scrollY > 40);
addEventListener("scroll", onScroll, { passive: true });
onScroll();

// Hero clips — only when the environment rules allow it (spec §5).
const hero = document.querySelector<HTMLElement>("[data-hero]");
if (hero) {
  fetch("/clips/clips.json")
    .then((r) => (r.ok ? r.json() : null))
    .then((json) => {
      const clips = parseClipsManifest(json) ?? [];
      const connection = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection;
      if (shouldPlayVideo({ reducedMotion, saveData: connection?.saveData === true, viewportWidth: innerWidth, clips })) {
        initHero(hero, clips);
      }
    })
    .catch(() => undefined); // poster stays
}

// Scroll reveals + the ledger sequence, each once.
const io = new IntersectionObserver(
  (entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      io.unobserve(e.target);
      e.target.classList.add("is-in");
      const ledger = e.target.querySelector<HTMLElement>("[data-ledger]");
      if (ledger) playLedger(ledger);
    }
  },
  { rootMargin: "0px 0px -10% 0px" },
);
for (const el of document.querySelectorAll("[data-reveal]")) io.observe(el);

function playLedger(ledger: HTMLElement) {
  const rows = Array.from(ledger.querySelectorAll<HTMLElement>(".ledger__row"));
  if (reducedMotion) {
    for (const r of rows) r.classList.add("is-in");
    return;
  }
  ledger.classList.add("ledger--armed");
  const delays = ledgerSchedule(rows.map((r) => ({ t: r.dataset.t ?? "0:00" })));
  rows.forEach((r, i) => setTimeout(() => r.classList.add("is-in"), delays[i]));
}

// Hero evidence chips: jump to their insight card and ring it like a focus.
for (const chip of document.querySelectorAll<HTMLAnchorElement>(".chip--evidence[data-target]")) {
  chip.addEventListener("click", () => {
    const card = document.getElementById(chip.dataset.target ?? "");
    if (!card) return;
    card.classList.add("is-target");
    setTimeout(() => card.classList.remove("is-target"), 1200);
  });
}
