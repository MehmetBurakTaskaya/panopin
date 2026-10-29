import { describe, expect, it } from 'vitest';
import { commit, redo, startHistory, undo } from './history';
import {
  arrowHead,
  bounds,
  boxFrom,
  cropRect,
  emptyLayer,
  fitMark,
  hitTest,
  moveMark,
  snapEnd,
  squareEnd,
  visibleArea,
  type Mark,
} from './layer';

const pen: Mark = { id: 'p', type: 'pen', color: '#000000', width: 4, points: [[0, 0], [10, 0], [10, 10]] };
const rect: Mark = { id: 'r', type: 'rect', color: '#ff0000', width: 4, x: 10, y: 10, w: 40, h: 20 };

describe('edit layer geometry', () => {
  it('builds arrow heads exactly like the Rust renderer', () => {
    // Same numbers as flatten.rs `arrow_heads_follow_the_shared_formula`.
    expect(arrowHead(0, 0, 100, 0, 5)).toEqual([[100, 0], [80, 9], [80, -9], [80, 0]]);
    expect(arrowHead(1, 1, 1.2, 1, 5)).toBeNull();
    expect(arrowHead(0, 0, 10, 0, 5)![3]).toEqual([2.5, 0]);
  });

  it('constrains boxes to squares and lines to 45 degree steps', () => {
    expect(squareEnd([0, 0], [10, -4])).toEqual([10, -10]);
    const [x, y] = snapEnd([0, 0], [10, 9]);
    expect(x).toBeCloseTo(y);
    expect(Math.hypot(x, y)).toBeCloseTo(Math.hypot(10, 9));
    expect(snapEnd([0, 0], [10, 1])[1]).toBeCloseTo(0);
    expect(boxFrom([10, 10], [0, 4])).toEqual({ x: 0, y: 4, w: 10, h: 6 });
  });

  it('hits strokes and outlines but not the inside of a rectangle', () => {
    expect(hitTest(pen, [5, 1], 0)).toBe(true);
    expect(hitTest(pen, [5, 6], 0)).toBe(false);
    expect(hitTest(rect, [10, 20], 0)).toBe(true);
    expect(hitTest(rect, [30, 20], 0)).toBe(false);
    const ellipse: Mark = { ...rect, id: 'e', type: 'ellipse' };
    expect(hitTest(ellipse, [10, 20], 1)).toBe(true);
    expect(hitTest(ellipse, [30, 20], 1)).toBe(false);
  });

  it('moves and resizes marks through their bounds', () => {
    expect(bounds(moveMark(pen, 5, 5))).toEqual({ x: 5, y: 5, w: 10, h: 10 });
    const grown = fitMark(rect, bounds(rect), { x: 0, y: 0, w: 80, h: 40 });
    expect(bounds(grown)).toEqual({ x: 0, y: 0, w: 80, h: 40 });
    const text: Mark = { id: 't', type: 'text', color: '#000000', size: 20, text: 'a', bitmap: '', x: 0, y: 0, w: 50, h: 25 };
    expect((fitMark(text, bounds(text), { x: 0, y: 0, w: 100, h: 50 }) as { size: number }).size).toBe(40);
  });

  it('clamps crops like Rust and falls back to the whole image', () => {
    expect(cropRect({ x: -5, y: 2.4, w: 50, h: 500 }, 40, 30)).toEqual({ x: 0, y: 2, w: 40, h: 28 });
    expect(cropRect({ x: 10, y: 10, w: 0.2, h: 5 }, 40, 30)).toBeNull();
    expect(visibleArea(null, 40, 30)).toEqual({ x: 0, y: 0, w: 40, h: 30 });
  });
});

describe('edit history', () => {
  it('undoes and redoes whole steps and forgets the future after a new change', () => {
    const a = emptyLayer();
    const b = { ...a, marks: [pen] };
    const c = { ...b, marks: [pen, rect] };
    let h = commit(commit(startHistory(a), b), c);
    h = undo(h);
    expect(h.present).toBe(b);
    h = redo(h);
    expect(h.present).toBe(c);
    h = undo(undo(h));
    expect(h.present).toBe(a);
    expect(undo(h)).toBe(h);
    h = commit(h, b);
    expect(redo(h)).toBe(h);
  });
});
