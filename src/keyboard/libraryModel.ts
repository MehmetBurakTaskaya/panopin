// Cursor and selection for the Library's grid and list. Pure; the grid is `columns` wide and the
// list is one column. Scenarios live in libraryModel.test.ts.

export interface LibraryModel {
  ids: number[];
  /** Index of the focused item, or null when the view is empty. */
  cursor: number | null;
  /** Selected ids in the order they were selected. */
  selection: number[];
  anchor: number | null;
  columns: number;
}

export type LibraryAction =
  | { type: 'setIds'; ids: number[] }
  | { type: 'setColumns'; columns: number }
  | { type: 'move'; dx: number; dy: number; extend?: boolean }
  | { type: 'edge'; end: 'first' | 'last'; extend?: boolean }
  | { type: 'page'; dir: -1 | 1; rows: number; extend?: boolean }
  | { type: 'selectAll' }
  | { type: 'click'; id: number; ctrl?: boolean; shift?: boolean }
  | { type: 'focus'; id: number };

export function emptyModel(columns = 1): LibraryModel {
  return { ids: [], cursor: null, selection: [], anchor: null, columns: Math.max(1, columns) };
}

function goTo(model: LibraryModel, index: number, extend = false): LibraryModel {
  if (model.ids.length === 0) return { ...model, cursor: null, selection: [], anchor: null };
  const cursor = Math.max(0, Math.min(model.ids.length - 1, index));
  const id = model.ids[cursor]!;
  if (!extend) return { ...model, cursor, selection: [id], anchor: id };
  const from = model.anchor === null ? cursor : Math.max(0, model.ids.indexOf(model.anchor));
  const [lo, hi] = from <= cursor ? [from, cursor] : [cursor, from];
  return { ...model, cursor, selection: model.ids.slice(lo, hi + 1) };
}

export function reduceLibrary(model: LibraryModel, action: LibraryAction): LibraryModel {
  switch (action.type) {
    case 'setIds': {
      // Stay on the same item when it is still there, else on the same position.
      const current = model.cursor === null ? null : model.ids[model.cursor];
      const kept = new Set(action.ids);
      const selection = model.selection.filter((id) => kept.has(id));
      const next = { ...model, ids: action.ids, selection };
      if (action.ids.length === 0) return { ...next, cursor: null, selection: [], anchor: null };
      if (current !== undefined && current !== null && kept.has(current)) {
        return { ...next, cursor: action.ids.indexOf(current) };
      }
      if (model.cursor === null) return { ...next, cursor: null };
      return goTo(next, model.cursor);
    }
    case 'setColumns': {
      // Same model when nothing changed, so a resize that keeps the columns renders nothing.
      const columns = Math.max(1, Math.floor(action.columns));
      return columns === model.columns ? model : { ...model, columns };
    }
    case 'move': {
      if (model.cursor === null) return goTo(model, 0);
      return goTo(model, model.cursor + action.dx + action.dy * model.columns, action.extend);
    }
    case 'edge':
      return goTo(model, action.end === 'first' ? 0 : model.ids.length - 1, action.extend);
    case 'page': {
      const step = Math.max(1, action.rows) * model.columns * action.dir;
      return goTo(model, (model.cursor ?? 0) + step, action.extend);
    }
    case 'selectAll':
      return { ...model, selection: [...model.ids] };
    case 'click': {
      const index = model.ids.indexOf(action.id);
      if (index < 0) return model;
      if (action.shift) return goTo(model, index, true);
      if (action.ctrl) {
        const selected = model.selection.includes(action.id);
        const selection = selected ? model.selection.filter((s) => s !== action.id) : [...model.selection, action.id];
        return { ...model, cursor: index, selection, anchor: action.id };
      }
      return goTo(model, index);
    }
    case 'focus': {
      const index = model.ids.indexOf(action.id);
      return index < 0 ? model : goTo(model, index);
    }
  }
}

/** The selection, or the focused item when nothing is selected. */
export function targets(model: LibraryModel): number[] {
  if (model.selection.length > 0) return model.selection;
  const id = model.cursor === null ? undefined : model.ids[model.cursor];
  return id === undefined ? [] : [id];
}
