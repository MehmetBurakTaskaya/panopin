// The editable layer over an image (D-050), mirroring src-tauri/src/media/annotations.rs.
// Coordinates are original image pixels; Rust draws the same geometry at full resolution.

export const LAYER_VERSION = 1;
/** The highlighter paints its colour at this opacity, as one shape. */
export const HIGHLIGHT_ALPHA = 0.35;

export interface Area {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Point = [number, number];

interface Stroked {
  id: string;
  color: string;
  width: number;
}

export type Mark =
  | (Stroked & { type: 'pen' | 'highlighter'; points: Point[] })
  | (Stroked & Area & { type: 'rect' | 'ellipse' })
  | (Stroked & { type: 'line' | 'arrow'; x1: number; y1: number; x2: number; y2: number })
  | (Area & { id: string; type: 'text'; color: string; size: number; text: string; bitmap: string })
  | (Area & { id: string; type: 'pixelate'; block: number });

export type MarkType = Mark['type'];

export interface EditLayer {
  version: number;
  crop: Area | null;
  marks: Mark[];
}

export const emptyLayer = (): EditLayer => ({ version: LAYER_VERSION, crop: null, marks: [] });

export const isEmptyLayer = (layer: EditLayer) => layer.crop === null && layer.marks.length === 0;

let counter = 0;
export const newMarkId = () => `m${Date.now().toString(36)}${(counter++).toString(36)}`;

/** A box from two corners, any drag direction. */
export function boxFrom([x1, y1]: Point, [x2, y2]: Point): Area {
  return { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
}

/** Shift: boxes become squares (the longer side wins), keeping the drag direction. */
export function squareEnd([x1, y1]: Point, [x2, y2]: Point): Point {
  const side = Math.max(Math.abs(x2 - x1), Math.abs(y2 - y1));
  return [x1 + Math.sign(x2 - x1 || 1) * side, y1 + Math.sign(y2 - y1 || 1) * side];
}

/** Shift: lines snap to 45° steps, keeping their length. */
export function snapEnd([x1, y1]: Point, [x2, y2]: Point): Point {
  const len = Math.hypot(x2 - x1, y2 - y1);
  const step = Math.PI / 4;
  const angle = Math.round(Math.atan2(y2 - y1, x2 - x1) / step) * step;
  return [x1 + Math.cos(angle) * len, y1 + Math.sin(angle) * len];
}

/** Same as Rust `arrow_head`: tip, left, right, and where the shaft ends inside the head. */
export function arrowHead(x1: number, y1: number, x2: number, y2: number, width: number): [Point, Point, Point, Point] | null {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.hypot(dx, dy);
  if (len < 1) return null;
  const head = Math.min(Math.max(width * 4, 14), len * 0.75);
  const half = head * 0.45;
  const ux = dx / len;
  const uy = dy / len;
  const bx = x2 - ux * head;
  const by = y2 - uy * head;
  return [
    [x2, y2],
    [bx - uy * half, by + ux * half],
    [bx + uy * half, by - ux * half],
    [bx, by],
  ];
}

/** The box a mark occupies, without its stroke width. */
export function bounds(mark: Mark): Area {
  switch (mark.type) {
    case 'pen':
    case 'highlighter': {
      const xs = mark.points.map((p) => p[0]);
      const ys = mark.points.map((p) => p[1]);
      return boxFrom([Math.min(...xs), Math.min(...ys)], [Math.max(...xs), Math.max(...ys)]);
    }
    case 'line':
    case 'arrow':
      return boxFrom([mark.x1, mark.y1], [mark.x2, mark.y2]);
    default:
      return { x: mark.x, y: mark.y, w: mark.w, h: mark.h };
  }
}

function distanceToSegment([px, py]: Point, [ax, ay]: Point, [bx, by]: Point): number {
  const dx = bx - ax;
  const dy = by - ay;
  const len2 = dx * dx + dy * dy;
  const t = len2 === 0 ? 0 : Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / len2));
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
}

const inside = ([px, py]: Point, a: Area, pad = 0) =>
  px >= a.x - pad && px <= a.x + a.w + pad && py >= a.y - pad && py <= a.y + a.h + pad;

/** Whether a point (image pixels) touches a mark; `slack` widens thin strokes for the pointer. */
export function hitTest(mark: Mark, point: Point, slack: number): boolean {
  switch (mark.type) {
    case 'pen':
    case 'highlighter': {
      const reach = mark.width / 2 + slack;
      if (mark.points.length === 1) return Math.hypot(point[0] - mark.points[0]![0], point[1] - mark.points[0]![1]) <= reach;
      return mark.points.some((p, i) => i > 0 && distanceToSegment(point, mark.points[i - 1]!, p) <= reach);
    }
    case 'line':
    case 'arrow':
      return distanceToSegment(point, [mark.x1, mark.y1], [mark.x2, mark.y2]) <= mark.width / 2 + slack;
    case 'rect': {
      const reach = mark.width / 2 + slack;
      return inside(point, mark, reach) && !inside(point, { x: mark.x + reach, y: mark.y + reach, w: mark.w - 2 * reach, h: mark.h - 2 * reach });
    }
    case 'ellipse': {
      const rx = mark.w / 2;
      const ry = mark.h / 2;
      if (rx <= 0 || ry <= 0) return inside(point, mark, mark.width / 2 + slack);
      const nx = (point[0] - (mark.x + rx)) / rx;
      const ny = (point[1] - (mark.y + ry)) / ry;
      const off = Math.abs(Math.hypot(nx, ny) - 1) * Math.min(rx, ry);
      return off <= mark.width / 2 + slack;
    }
    default:
      return inside(point, mark, slack);
  }
}

export function moveMark<M extends Mark>(mark: M, dx: number, dy: number): M {
  switch (mark.type) {
    case 'pen':
    case 'highlighter':
      return { ...mark, points: mark.points.map(([x, y]) => [x + dx, y + dy] as Point) };
    case 'line':
    case 'arrow':
      return { ...mark, x1: mark.x1 + dx, y1: mark.y1 + dy, x2: mark.x2 + dx, y2: mark.y2 + dy };
    default:
      return { ...mark, x: mark.x + dx, y: mark.y + dy };
  }
}

/** Maps a mark from one box to another (resizing with the 8 handles). Text scales its size. */
export function fitMark<M extends Mark>(mark: M, from: Area, to: Area): M {
  const sx = from.w === 0 ? 1 : to.w / from.w;
  const sy = from.h === 0 ? 1 : to.h / from.h;
  const map = ([x, y]: Point): Point => [to.x + (x - from.x) * sx, to.y + (y - from.y) * sy];
  switch (mark.type) {
    case 'pen':
    case 'highlighter':
      return { ...mark, points: mark.points.map(map) };
    case 'line':
    case 'arrow': {
      const [x1, y1] = map([mark.x1, mark.y1]);
      const [x2, y2] = map([mark.x2, mark.y2]);
      return { ...mark, x1, y1, x2, y2 };
    }
    case 'text':
      return { ...mark, x: to.x, y: to.y, w: to.w, h: to.h, size: mark.size * Math.min(sx, sy) };
    default:
      return { ...mark, x: to.x, y: to.y, w: to.w, h: to.h };
  }
}

/** A crop as whole pixels inside the image, like Rust `crop_rect`; null when nothing is left. */
export function cropRect(crop: Area, width: number, height: number): Area | null {
  const clamp = (v: number, max: number) => Math.max(0, Math.min(max, Math.round(v)));
  const left = clamp(crop.x, width);
  const top = clamp(crop.y, height);
  const right = clamp(crop.x + crop.w, width);
  const bottom = clamp(crop.y + crop.h, height);
  return right > left && bottom > top ? { x: left, y: top, w: right - left, h: bottom - top } : null;
}

/** The part of the image that is shown: the crop, or all of it. */
export function visibleArea(layer: EditLayer | null, width: number, height: number): Area {
  return (layer?.crop && cropRect(layer.crop, width, height)) || { x: 0, y: 0, w: width, h: height };
}
