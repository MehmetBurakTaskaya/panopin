// Draws an edit layer for the screen with Canvas 2D, using the same geometry as the Rust renderer
// (src-tauri/src/media/flatten.rs). The caller sets the canvas transform so that drawing happens
// in image pixels.
import { HIGHLIGHT_ALPHA, arrowHead, bounds, cropRect, type Area, type Mark, type Point } from './layer';

/** Image pixels to canvas pixels: canvas = image × scale + offset. */
export interface View {
  scale: number;
  ox: number;
  oy: number;
}

export const toImage = (view: View, x: number, y: number): Point => [(x - view.ox) / view.scale, (y - view.oy) / view.scale];
export const toCanvas = (view: View, x: number, y: number): Point => [x * view.scale + view.ox, y * view.scale + view.oy];

export function applyView(ctx: CanvasRenderingContext2D, view: View, dpr: number): void {
  ctx.setTransform(view.scale * dpr, 0, 0, view.scale * dpr, view.ox * dpr, view.oy * dpr);
}

// Text marks carry a PNG drawn by the webview; decode each once.
const decoded = new Map<string, HTMLImageElement>();

function bitmapOf(src: string, onReady: () => void): HTMLImageElement | null {
  let image = decoded.get(src);
  if (!image) {
    image = new Image();
    image.onload = onReady;
    image.src = src;
    decoded.set(src, image);
    if (decoded.size > 200) decoded.delete(decoded.keys().next().value!);
  }
  return image.complete && image.naturalWidth > 0 ? image : null;
}

export function drawMark(ctx: CanvasRenderingContext2D, mark: Mark, onBitmap: () => void): void {
  ctx.save();
  switch (mark.type) {
    case 'pen':
    case 'highlighter': {
      ctx.globalAlpha = mark.type === 'highlighter' ? HIGHLIGHT_ALPHA : 1;
      ctx.strokeStyle = mark.color;
      ctx.fillStyle = mark.color;
      const [first, ...rest] = mark.points;
      if (!first) break;
      if (rest.length === 0) {
        ctx.beginPath();
        ctx.arc(first[0], first[1], mark.width / 2, 0, Math.PI * 2);
        ctx.fill();
        break;
      }
      // One path: a stroke that crosses itself keeps one even tint.
      ctx.lineWidth = mark.width;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      ctx.beginPath();
      ctx.moveTo(first[0], first[1]);
      for (const p of rest) ctx.lineTo(p[0], p[1]);
      ctx.stroke();
      break;
    }
    case 'rect':
    case 'ellipse':
      ctx.strokeStyle = mark.color;
      ctx.lineWidth = mark.width;
      ctx.lineJoin = 'miter';
      ctx.lineCap = 'butt';
      ctx.beginPath();
      if (mark.type === 'rect') ctx.rect(mark.x, mark.y, mark.w, mark.h);
      else ctx.ellipse(mark.x + mark.w / 2, mark.y + mark.h / 2, mark.w / 2, mark.h / 2, 0, 0, Math.PI * 2);
      ctx.stroke();
      break;
    case 'line':
      ctx.strokeStyle = mark.color;
      ctx.lineWidth = mark.width;
      ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.moveTo(mark.x1, mark.y1);
      ctx.lineTo(mark.x2, mark.y2);
      ctx.stroke();
      break;
    case 'arrow': {
      const head = arrowHead(mark.x1, mark.y1, mark.x2, mark.y2, mark.width);
      if (!head) break;
      const [tip, left, right, base] = head;
      ctx.strokeStyle = mark.color;
      ctx.fillStyle = mark.color;
      ctx.lineWidth = mark.width;
      ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.moveTo(mark.x1, mark.y1);
      ctx.lineTo(base[0], base[1]);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(tip[0], tip[1]);
      ctx.lineTo(left[0], left[1]);
      ctx.lineTo(right[0], right[1]);
      ctx.closePath();
      ctx.fill();
      break;
    }
    case 'text': {
      const image = bitmapOf(mark.bitmap, onBitmap);
      if (image) ctx.drawImage(image, mark.x, mark.y, mark.w, mark.h);
      break;
    }
    case 'pixelate':
      // Drawn into the base picture, under every other mark (see drawPixelations).
      break;
  }
  ctx.restore();
}

export function drawMarks(ctx: CanvasRenderingContext2D, marks: readonly Mark[], onBitmap: () => void): void {
  for (const mark of marks) drawMark(ctx, mark, onBitmap);
}

// Small block images per pixelated area, keyed by source and geometry.
const blockCache = new WeakMap<object, Map<string, HTMLCanvasElement>>();

/** Pixelated areas for the screen. The blocks approximate averages with a smooth downscale;
 * the copied result uses exact averages computed in Rust. */
export function drawPixelations(
  ctx: CanvasRenderingContext2D,
  source: ImageBitmap | HTMLImageElement,
  width: number,
  height: number,
  marks: readonly Mark[],
): void {
  let cache = blockCache.get(source);
  if (!cache) {
    cache = new Map();
    blockCache.set(source, cache);
  }
  for (const mark of marks) {
    if (mark.type !== 'pixelate') continue;
    const area = cropRect(mark, width, height);
    if (!area) continue;
    const block = Math.max(2, Math.round(mark.block));
    const cols = Math.ceil(area.w / block);
    const rows = Math.ceil(area.h / block);
    const key = `${area.x},${area.y},${area.w},${area.h},${block}`;
    let small = cache.get(key);
    if (!small) {
      small = document.createElement('canvas');
      small.width = cols;
      small.height = rows;
      const sctx = small.getContext('2d')!;
      sctx.imageSmoothingQuality = 'high';
      sctx.drawImage(source, area.x, area.y, cols * block, rows * block, 0, 0, cols, rows);
      cache.set(key, small);
    }
    ctx.save();
    ctx.beginPath();
    ctx.rect(area.x, area.y, area.w, area.h);
    ctx.clip();
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(small, area.x, area.y, cols * block, rows * block);
    ctx.restore();
  }
}

export type Handle = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w' | 'p1' | 'p2';

/** Grab points of a selected mark in image pixels: line ends, or the 8 points of its box. */
export function handlesOf(mark: Mark): [Handle, Point][] {
  if (mark.type === 'line' || mark.type === 'arrow') {
    return [
      ['p1', [mark.x1, mark.y1]],
      ['p2', [mark.x2, mark.y2]],
    ];
  }
  return boxHandles(bounds(mark));
}

export function boxHandles(b: Area): [Handle, Point][] {
  const { x, y, w, h } = b;
  return [
    ['nw', [x, y]],
    ['n', [x + w / 2, y]],
    ['ne', [x + w, y]],
    ['e', [x + w, y + h / 2]],
    ['se', [x + w, y + h]],
    ['s', [x + w / 2, y + h]],
    ['sw', [x, y + h]],
    ['w', [x, y + h / 2]],
  ];
}

/** Drags one handle of a box; the opposite side stays put. */
export function dragBox(box: Area, handle: Handle, [px, py]: Point): Area {
  let left = box.x;
  let top = box.y;
  let right = box.x + box.w;
  let bottom = box.y + box.h;
  if (handle.includes('w')) left = px;
  if (handle.includes('e')) right = px;
  if (handle.includes('n')) top = py;
  if (handle.includes('s')) bottom = py;
  return { x: Math.min(left, right), y: Math.min(top, bottom), w: Math.abs(right - left), h: Math.abs(bottom - top) };
}

/** The dashed box and grab points of the selection, in screen pixels so they keep their size. */
export function drawSelection(ctx: CanvasRenderingContext2D, mark: Mark, view: View, dpr: number, accent: string): void {
  ctx.save();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.strokeStyle = accent;
  ctx.lineWidth = 1;
  if (mark.type !== 'line' && mark.type !== 'arrow') {
    const b = bounds(mark);
    const [x1, y1] = toCanvas(view, b.x, b.y);
    const [x2, y2] = toCanvas(view, b.x + b.w, b.y + b.h);
    ctx.setLineDash([4, 3]);
    ctx.strokeRect(x1 - 4, y1 - 4, x2 - x1 + 8, y2 - y1 + 8);
    ctx.setLineDash([]);
  }
  for (const [, [hx, hy]] of handlesOf(mark)) {
    const [cx, cy] = toCanvas(view, hx, hy);
    ctx.fillStyle = '#fff';
    ctx.fillRect(cx - 4, cy - 4, 8, 8);
    ctx.strokeRect(cx - 4, cy - 4, 8, 8);
  }
  ctx.restore();
}

/** Renders a text mark's glyphs at image resolution, as the PNG stored in the layer. */
export function renderTextBitmap(text: string, size: number, color: string): { bitmap: string; w: number; h: number } {
  const font = `600 ${size}px 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif`;
  const lines = text.split('\n');
  const lineHeight = Math.ceil(size * 1.25);
  const pad = Math.ceil(size * 0.15);
  const measure = document.createElement('canvas').getContext('2d')!;
  measure.font = font;
  const width = Math.max(1, ...lines.map((line) => Math.ceil(measure.measureText(line).width)));
  const canvas = document.createElement('canvas');
  canvas.width = width + pad * 2;
  canvas.height = lineHeight * lines.length + pad * 2;
  const ctx = canvas.getContext('2d')!;
  ctx.font = font;
  ctx.fillStyle = color;
  ctx.textBaseline = 'top';
  lines.forEach((line, i) => ctx.fillText(line, pad, pad + i * lineHeight + (lineHeight - size) / 2));
  return { bitmap: canvas.toDataURL('image/png'), w: canvas.width, h: canvas.height };
}
