import { useEffect, useRef } from "react";
import { rgba } from "../lib/theme";
import { radarImageUrl } from "../replay/coords";
import { cellRect, densityToAlpha, gridMax } from "../replay/heatmap";
import type { GridDto } from "../replay/heatmap";

interface Props {
  grid: GridDto | null;
  map: string;
}

const CANVAS_PX = 512;

/** Draws the pro-corpus occupancy grid over the map radar: one sequential
 *  hue (CT blue, read from the --ct token via theme.ts) at density-driven
 *  alpha, no per-cell borders or labels — this is a magnitude encoding, not
 *  a categorical one. CT blue is correct here (spec §9): corpus grids are
 *  per-side data, not chrome. */
export function HeatmapCanvas({ grid, map }: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !grid) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Guards against a stale, still-pending image load from a previous
    // run of this effect (rapid grid/map switching can leave an older,
    // slower-loading image racing a newer one) — without this, a late
    // onload could clearRect + draw with the OLD grid over the current one.
    let stale = false;

    const drawCells = () => {
      const max = gridMax(grid.counts);
      for (let index = 0; index < grid.counts.length; index++) {
        const count = grid.counts[index];
        if (count <= 0) continue;
        const alpha = densityToAlpha(count, max);
        const { x, y, w, h } = cellRect(index, grid.size, CANVAS_PX);
        ctx.fillStyle = rgba("--ct", alpha);
        ctx.fillRect(x, y, w, h);
      }
    };

    // onload is assigned before src, so the load event fires asynchronously
    // even for a cached image (per the HTML spec) — one draw path suffices.
    const image = new Image();
    image.onload = () => {
      if (stale) return;
      ctx.clearRect(0, 0, CANVAS_PX, CANVAS_PX);
      ctx.drawImage(image, 0, 0, CANVAS_PX, CANVAS_PX);
      drawCells();
    };
    image.src = radarImageUrl(map, "upper");

    return () => {
      stale = true;
    };
  }, [grid, map]);

  if (!grid || grid.demos === 0) {
    return (
      <div className="cps-heatmap-well cps-heatmap-well-empty">
        <p className="type-body cps-heatmap-empty">
          No corpus data for this map yet.
        </p>
      </div>
    );
  }

  return (
    <div className="cps-heatmap-well">
      <canvas
        ref={canvasRef}
        className="cps-heatmap-canvas"
        width={CANVAS_PX}
        height={CANVAS_PX}
        role="img"
        aria-label={`Pro-corpus occupancy heatmap for ${map}`}
      />
      <p className="type-data cps-heatmap-caption">
        {grid.demos} demos · {grid.samples} samples
      </p>
    </div>
  );
}
