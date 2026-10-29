import { useEffect, useLayoutEffect, useMemo, useRef, useState, type HTMLAttributes, type Ref, type RefObject, type WheelEvent } from 'react';
import { applyView, drawMarks, drawPixelations, toImage, type View } from '../../../edit/draw';
import type { Area, EditLayer } from '../../../edit/layer';
import styles from './Preview.module.css';

export interface Size {
  w: number;
  h: number;
}

/** The CSS size of an element, kept current. */
export function useElementSize(ref: RefObject<HTMLElement | null>): Size {
  const [size, setSize] = useState<Size>({ w: 0, h: 0 });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return size;
}

const MARGIN = 24;
export const MIN_ZOOM = 1;
export const MAX_ZOOM = 4;

/** Fits `area` into the stage (never enlarging small pictures), then applies zoom and pan. */
export function fitView(size: Size, area: Area, zoom: number, pan: [number, number]): View {
  const fit = Math.min(1, (size.w - MARGIN * 2) / area.w, (size.h - MARGIN * 2) / area.h);
  const scale = Math.max(0.01, fit) * zoom;
  return {
    scale,
    ox: (size.w - area.w * scale) / 2 - area.x * scale + pan[0],
    oy: (size.h - area.h * scale) / 2 - area.y * scale + pan[1],
  };
}

/** Zooming keeps the image point under the pointer in place; zoom 1 always recentres. */
export function zoomAt(size: Size, area: Area, view: View, zoom: number, at: [number, number]): [number, [number, number]] {
  const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
  if (next === MIN_ZOOM) return [next, [0, 0]];
  const [px, py] = toImage(view, at[0], at[1]);
  const centred = fitView(size, area, next, [0, 0]);
  return [next, [at[0] - (px * centred.scale + centred.ox), at[1] - (py * centred.scale + centred.oy)]];
}

/** Measures a stage container and fits `area` into it: the view to draw with and to map pointers. */
export function useFittedView(area: Area, zoom: number, pan: [number, number]) {
  const box = useRef<HTMLDivElement>(null);
  const size = useElementSize(box);
  const view = useMemo(() => fitView(size, area, zoom, pan), [size, area, zoom, pan]);
  return { box, size, view };
}

/** Wheel zoom (100–400 % of the fitted size) around the pointer. */
export function wheelZoom(
  e: WheelEvent<HTMLElement>,
  size: Size,
  area: Area,
  view: View,
  zoom: number,
  apply: (zoom: number, pan: [number, number]) => void,
): void {
  const rect = e.currentTarget.getBoundingClientRect();
  const [next, pan] = zoomAt(size, area, view, zoom * Math.exp(-e.deltaY * 0.0015), [e.clientX - rect.left, e.clientY - rect.top]);
  apply(next, pan);
}

export interface StageProps extends HTMLAttributes<HTMLDivElement> {
  ref?: Ref<HTMLDivElement>;
  image: ImageBitmap;
  layer: EditLayer;
  view: View;
  size: Size;
  /** The part of the image to show (the crop), or null for all of it. */
  clip: Area | null;
  /** Drawing over the marks in canvas pixels: the stroke in progress, the selection, the crop frame. */
  overlay?: (ctx: CanvasRenderingContext2D, dpr: number) => void;
  /** Changes when the overlay must be drawn again although nothing else changed. */
  overlayKey?: unknown;
}

function fitCanvas(canvas: HTMLCanvasElement, size: Size, dpr: number) {
  const w = Math.max(1, Math.round(size.w * dpr));
  const h = Math.max(1, Math.round(size.h * dpr));
  if (canvas.width !== w) canvas.width = w;
  if (canvas.height !== h) canvas.height = h;
}

/**
 * Two canvases: the decoded picture with its pixelated areas (redrawn only when the view or those
 * areas change), and the marks on top (redrawn once per animation frame while drawing).
 */
export function Stage({ ref, image, layer, view, size, clip, overlay, overlayKey, className, ...rest }: StageProps) {
  const base = useRef<HTMLCanvasElement>(null);
  const top = useRef<HTMLCanvasElement>(null);
  const frame = useRef(0);
  const [bitmapsReady, setBitmapsReady] = useState(0);
  const dpr = window.devicePixelRatio || 1;

  useLayoutEffect(() => {
    const canvas = base.current;
    const ctx = canvas?.getContext('2d');
    if (!canvas || !ctx) return;
    fitCanvas(canvas, size, dpr);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    applyView(ctx, view, dpr);
    if (clip) {
      ctx.beginPath();
      ctx.rect(clip.x, clip.y, clip.w, clip.h);
      ctx.clip();
    }
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(image, 0, 0);
    drawPixelations(ctx, image, image.width, image.height, layer.marks);
  }, [image, layer.marks, view, size, dpr, clip]);

  useEffect(() => {
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(() => {
      const canvas = top.current;
      const ctx = canvas?.getContext('2d');
      if (!canvas || !ctx) return;
      fitCanvas(canvas, size, dpr);
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      ctx.save();
      applyView(ctx, view, dpr);
      if (clip) {
        ctx.beginPath();
        ctx.rect(clip.x, clip.y, clip.w, clip.h);
        ctx.clip();
      }
      drawMarks(ctx, layer.marks, () => setBitmapsReady((n) => n + 1));
      ctx.restore();
      overlay?.(ctx, dpr);
    });
    return () => cancelAnimationFrame(frame.current);
  }, [layer.marks, view, size, dpr, clip, overlay, overlayKey, bitmapsReady]);

  return (
    <div ref={ref} className={`${styles.stage} ${className ?? ''}`} {...rest}>
      <canvas ref={base} aria-hidden="true" />
      <canvas ref={top} aria-hidden="true" />
    </div>
  );
}
