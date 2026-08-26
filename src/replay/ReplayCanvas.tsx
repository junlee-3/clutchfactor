import { useEffect, useRef } from "react";
import { draw, LOGICAL_W } from "./Renderer";
import type { Scene } from "./Renderer";

interface Props {
  /** Called each frame to obtain the current scene (reads live tick refs).
   *  `cssWidth` is the canvas's current `clientWidth` — ReplayCanvas reads it
   *  once per frame (cheap: a layout read already cached by the browser
   *  between resizes) so Scene.cssWidth stays live without a ResizeObserver. */
  getScene: (cssWidth: number) => Scene;
  /** Called each frame with dt (seconds) so the parent can advance time. */
  onFrame: (dtSeconds: number) => void;
  onFps?: (fps: number) => void;
}

export function ReplayCanvas({ getScene, onFrame, onFps }: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  // Latest callbacks without re-arming the rAF loop.
  const callbacksRef = useRef({ getScene, onFrame, onFps });
  useEffect(() => {
    callbacksRef.current = { getScene, onFrame, onFps };
  }, [getScene, onFrame, onFps]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = LOGICAL_W * dpr;
    canvas.height = LOGICAL_W * dpr;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    let raf = 0;
    let last = performance.now();
    let frames = 0;
    let fpsWindowStart = last;

    const loop = (now: number) => {
      const { getScene, onFrame, onFps } = callbacksRef.current;
      const dt = Math.min((now - last) / 1000, 0.25);
      last = now;
      onFrame(dt);
      draw(ctx, getScene(canvas.clientWidth));

      frames++;
      if (now - fpsWindowStart >= 1000) {
        onFps?.(Math.round((frames * 1000) / (now - fpsWindowStart)));
        frames = 0;
        fpsWindowStart = now;
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="rpl-canvas"
      role="img"
      aria-label="2D round replay"
    />
  );
}
