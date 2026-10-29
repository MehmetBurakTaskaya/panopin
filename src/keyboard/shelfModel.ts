// The shelf's keyboard and selection model. Pure: React only dispatches actions to it, and the
// scenarios in shelfModel.test.ts pin its behaviour down.

export type Kind = 'image' | 'text' | 'link';
export const KINDS: readonly Kind[] = ['image', 'text', 'link'];

/** Item ids per row, in display order (hidden rows and filters already applied by the caller). */
export type Rows = Record<Kind, number[]>;

export interface Cursor {
  kind: Kind;
  index: number;
}

export interface ShelfModel {
  rows: Rows;
  cursor: Cursor | null;
  /** Selected ids in the order they were selected (multi-copy keeps this order). */
  selection: number[];
  anchor: number | null;
  /** Cards visible at once in a row, at most 9; drives paging and the 1–9 badges. */
  pageSize: number;
}

export type Action =
  | { type: 'move'; dx: -1 | 1; extend?: boolean }
  | { type: 'row'; dy: -1 | 1 }
  | { type: 'tab'; back: boolean }
  | { type: 'edge'; end: 'first' | 'last' }
  | { type: 'page'; dir: -1 | 1 }
  | { type: 'pick'; n: number }
  | { type: 'selectAll' }
  | { type: 'click'; id: number; ctrl?: boolean; shift?: boolean }
  | { type: 'setRows'; rows: Rows }
  | { type: 'setPageSize'; pageSize: number };

export const filledKinds = (rows: Rows): Kind[] => KINDS.filter((kind) => rows[kind].length > 0);

const idAt = (rows: Rows, cursor: Cursor | null): number | null =>
  cursor ? (rows[cursor.kind][cursor.index] ?? null) : null;

const clamp = (value: number, max: number) => Math.max(0, Math.min(value, max));

/** On open: the first image, or else the first item of the first filled row. */
export function openModel(rows: Rows, pageSize = 9): ShelfModel {
  const kind = rows.image.length > 0 ? 'image' : filledKinds(rows)[0];
  const cursor = kind ? { kind, index: 0 } : null;
  const id = idAt(rows, cursor);
  return { rows, cursor, selection: id === null ? [] : [id], anchor: id, pageSize: clampPage(pageSize) };
}

const clampPage = (size: number) => Math.max(1, Math.min(9, Math.floor(size)));

/** First index of the page that holds the cursor. */
export function pageStart(model: ShelfModel): number {
  return model.cursor ? model.cursor.index - (model.cursor.index % model.pageSize) : 0;
}

function focus(model: ShelfModel, cursor: Cursor | null): ShelfModel {
  const id = idAt(model.rows, cursor);
  return { ...model, cursor, selection: id === null ? [] : [id], anchor: id };
}

function moveTo(model: ShelfModel, kind: Kind, index: number): ShelfModel {
  return focus(model, { kind, index: clamp(index, model.rows[kind].length - 1) });
}

export function reduce(model: ShelfModel, action: Action): ShelfModel {
  const { rows, cursor } = model;
  if (action.type === 'setRows') return keepCursor(model, action.rows);
  if (action.type === 'setPageSize') return { ...model, pageSize: clampPage(action.pageSize) };
  if (action.type === 'click') return click(model, action.id, action.ctrl, action.shift);
  if (!cursor) return model;
  const length = rows[cursor.kind].length;
  const filled = filledKinds(rows);
  const row = filled.indexOf(cursor.kind);

  switch (action.type) {
    case 'move': {
      const index = clamp(cursor.index + action.dx, length - 1);
      if (!action.extend) return moveTo(model, cursor.kind, index);
      return extendTo(model, index);
    }
    case 'row': {
      // Up/down skips empty rows and keeps the column; a shorter row clamps to its last card.
      const target = filled[row + action.dy];
      return target ? moveTo(model, target, cursor.index) : model;
    }
    case 'tab': {
      const target = filled[(row + (action.back ? -1 : 1) + filled.length) % filled.length];
      return target ? moveTo(model, target, cursor.index) : model;
    }
    case 'edge':
      return moveTo(model, cursor.kind, action.end === 'first' ? 0 : length - 1);
    case 'page':
      return moveTo(model, cursor.kind, pageStart(model) + action.dir * model.pageSize);
    case 'pick': {
      const index = pageStart(model) + action.n - 1;
      return action.n >= 1 && action.n <= model.pageSize && index < length ? moveTo(model, cursor.kind, index) : model;
    }
    case 'selectAll':
      return { ...model, selection: [...rows[cursor.kind]] };
  }
}

/** Shift+arrows: the selection becomes the range from the anchor to the new cursor, in row order. */
function extendTo(model: ShelfModel, index: number): ShelfModel {
  const kind = model.cursor!.kind;
  const ids = model.rows[kind];
  const anchorIndex = model.anchor === null ? -1 : ids.indexOf(model.anchor);
  const from = anchorIndex < 0 ? index : anchorIndex;
  const [lo, hi] = from <= index ? [from, index] : [index, from];
  return { ...model, cursor: { kind, index }, selection: ids.slice(lo, hi + 1) };
}

function click(model: ShelfModel, id: number, ctrl = false, shift = false): ShelfModel {
  const kind = KINDS.find((k) => model.rows[k].includes(id));
  if (!kind) return model;
  const index = model.rows[kind].indexOf(id);
  if (shift && model.cursor?.kind === kind) return extendTo(model, index);
  if (ctrl) {
    const selected = model.selection.includes(id);
    const selection = selected ? model.selection.filter((s) => s !== id) : [...model.selection, id];
    return { ...model, cursor: { kind, index }, selection, anchor: id };
  }
  return focus(model, { kind, index });
}

/** After the rows change (capture, take down, undo): stay on the same item if it is still there,
 * otherwise on the same position, which is the next item; selection keeps what still exists. */
function keepCursor(model: ShelfModel, rows: Rows): ShelfModel {
  const current = idAt(model.rows, model.cursor);
  const exists = (id: number) => KINDS.some((k) => rows[k].includes(id));
  const selection = model.selection.filter(exists);
  if (current !== null && exists(current)) {
    const kind = KINDS.find((k) => rows[k].includes(current))!;
    return { ...model, rows, cursor: { kind, index: rows[kind].indexOf(current) }, selection };
  }
  const next: ShelfModel = { ...model, rows };
  if (model.cursor && rows[model.cursor.kind].length > 0) {
    return moveTo(next, model.cursor.kind, model.cursor.index);
  }
  const fallback = filledKinds(rows)[0];
  return fallback ? moveTo(next, fallback, 0) : { ...next, cursor: null, selection: [], anchor: null };
}
