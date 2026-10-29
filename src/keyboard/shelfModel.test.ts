import { describe, expect, it } from 'vitest';
import { openModel, pageStart, reduce, type Action, type Rows, type ShelfModel } from './shelfModel';

const rows = (image: number[], text: number[], link: number[]): Rows => ({ image, text, link });
const run = (model: ShelfModel, ...actions: Action[]) => actions.reduce(reduce, model);
const at = (model: ShelfModel) => model.cursor && `${model.cursor.kind}:${model.cursor.index}`;

describe('shelf keyboard model', () => {
  it('opens on the first image', () => {
    const model = openModel(rows([1, 2], [10], [20]));
    expect(at(model)).toBe('image:0');
    expect(model.selection).toEqual([1]);
  });

  it('opens on the first item of the first filled row when there are no images', () => {
    expect(at(openModel(rows([], [], [20, 21])))).toBe('link:0');
    expect(openModel(rows([], [], [])).cursor).toBeNull();
  });

  it('moves down to the next filled row and keeps the column', () => {
    const model = run(openModel(rows([1, 2, 3], [10, 11, 12], [20])), { type: 'move', dx: 1 }, { type: 'move', dx: 1 });
    expect(at(run(model, { type: 'row', dy: 1 }))).toBe('text:2');
  });

  it('clamps to the last card of a shorter row', () => {
    const model = run(openModel(rows([1, 2, 3], [10], [20])), { type: 'edge', end: 'last' }, { type: 'row', dy: 1 });
    expect(at(model)).toBe('text:0');
  });

  it('skips empty rows', () => {
    const model = run(openModel(rows([1], [], [20])), { type: 'row', dy: 1 });
    expect(at(model)).toBe('link:0');
    expect(at(run(model, { type: 'row', dy: 1 }))).toBe('link:0');
  });

  it('starts fresh every time the shelf opens', () => {
    const data = rows([1, 2], [10], []);
    const moved = run(openModel(data), { type: 'row', dy: 1 });
    expect(at(moved)).toBe('text:0');
    expect(at(openModel(data))).toBe('image:0');
  });

  it('stops at the ends and wraps rows with Tab', () => {
    const model = openModel(rows([1, 2], [10], [20]));
    expect(at(run(model, { type: 'move', dx: -1 }))).toBe('image:0');
    expect(at(run(model, { type: 'tab', back: true }))).toBe('link:0');
    expect(at(run(model, { type: 'tab', back: false }, { type: 'tab', back: false }, { type: 'tab', back: false }))).toBe('image:0');
  });

  it('pages and picks numbered cards within the page', () => {
    const many = rows(Array.from({ length: 20 }, (_, i) => i + 1), [], []);
    const model = openModel(many, 5);
    const paged = run(model, { type: 'page', dir: 1 });
    expect(at(paged)).toBe('image:5');
    expect(pageStart(paged)).toBe(5);
    expect(at(run(paged, { type: 'pick', n: 3 }))).toBe('image:7');
    expect(at(run(paged, { type: 'pick', n: 6 }))).toBe('image:5'); // beyond the page size: ignored
    expect(at(run(paged, { type: 'page', dir: -1 }))).toBe('image:0');
  });

  it('extends the selection with Shift and keeps the click order with Ctrl', () => {
    const model = openModel(rows([], [10, 11, 12, 13], []));
    expect(run(model, { type: 'move', dx: 1, extend: true }, { type: 'move', dx: 1, extend: true }).selection).toEqual([10, 11, 12]);
    const clicked = run(model, { type: 'click', id: 13, ctrl: true }, { type: 'click', id: 11, ctrl: true });
    expect(clicked.selection).toEqual([10, 13, 11]);
    expect(run(clicked, { type: 'click', id: 13, ctrl: true }).selection).toEqual([10, 11]);
    expect(run(model, { type: 'selectAll' }).selection).toEqual([10, 11, 12, 13]);
  });

  it('lands on the next card after the current one is taken down', () => {
    const model = run(openModel(rows([1, 2, 3], [], [])), { type: 'move', dx: 1 });
    const after = reduce(model, { type: 'setRows', rows: rows([1, 3], [], []) });
    expect(at(after)).toBe('image:1');
    expect(after.selection).toEqual([3]);
    const lastGone = reduce(after, { type: 'setRows', rows: rows([1], [], []) });
    expect(at(lastGone)).toBe('image:0');
  });

  it('follows its card when new items push it along', () => {
    const model = run(openModel(rows([1, 2], [], [])), { type: 'move', dx: 1 });
    const after = reduce(model, { type: 'setRows', rows: rows([9, 1, 2], [], []) });
    expect(at(after)).toBe('image:2');
  });

  it('moves to another row when its row empties', () => {
    const model = openModel(rows([1], [10], []));
    expect(at(reduce(model, { type: 'setRows', rows: rows([], [10], []) }))).toBe('text:0');
    expect(reduce(model, { type: 'setRows', rows: rows([], [], []) }).cursor).toBeNull();
  });
});
