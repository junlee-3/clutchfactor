import { useEffect, useRef } from "react";
import { radarImageUrl } from "../replay/coords";
import { cellRect, densityToAlpha, gridMax } from "../replay/heatmap";
import type { GridDto } from "../replay/heatmap";

interface Props {
  grid: GridDto | null;
  map: string;
}

const CANVAS_PX = 512;
const CT_BLUE = "74, 163, 255"; // #4aa3ff — single sequential hue (dataviz: magnitude, no rainbow)

/** Draws the pro-corpus occupancy grid over the map radar: one sequential
 *  hue (CT blue) at density-driven alpha, no per-cell borders or labels —
 *  this is a magnitude encoding, not a categorical one. */
export function HeatmapCanvas({ grid, map }: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !grid) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const drawCells = () => {
      const max = gridMax(grid.counts);
      for (let index = 0; index < grid.counts.length; index++) {
        const count = grid.counts[index];
        if (count <= 0) continue;
        const alpha = densityToAlpha(count, max);
        const { x, y, w, h } = cellRect(index, grid.size, CANVAS_PX);
        ctx.fillStyle = `rgba(${CT_BLUE}, ${alpha})`;
        ctx.fillRect(x, y, w, h);
      }
    };

    // onload is assigned before src, so the load event fires asynchronously
    // even for a cached image (per the HTML spec) — one draw path suffices.
    const image = new Image();
    image.onload = () => {
      ctx.clearRect(0, 0, CANVAS_PX, CANVAS_PX);
      ctx.drawImage(image, 0, 0, CANVAS_PX, CANVAS_PX);
      drawCells();
    };
    image.src = radarImageUrl(map, "upper");
  }, [grid, map]);

  if (!grid || grid.demos === 0) {
    return (
      <div className="heatmap-canvas-wrap">
        <p className="heatmap-empty">No corpus data for this map yet.</p>
      </div>
    );
  }

  return (
    <div className="heatmap-canvas-wrap">
      <canvas
        ref={canvasRef}
        className="heatmap-canvas"
        width={CANVAS_PX}
        height={CANVAS_PX}
        role="img"
        aria-label={`Pro-corpus occupancy heatmap for ${map}`}
      />
      <p className="heatmap-caption">
        {grid.demos} demos · {grid.samples} samples
      </p>
    </div>
  );
}
