// Undo and redo for the markup editor: whole-layer snapshots, so every step is exact.
import type { EditLayer } from './layer';

const LIMIT = 200;

export interface History {
  past: EditLayer[];
  present: EditLayer;
  future: EditLayer[];
}

export const startHistory = (layer: EditLayer): History => ({ past: [], present: layer, future: [] });

/** A finished change (a stroke, a move, a deletion) becomes one undo step. */
export function commit(history: History, next: EditLayer): History {
  if (next === history.present) return history;
  return { past: [...history.past, history.present].slice(-LIMIT), present: next, future: [] };
}

export function undo(history: History): History {
  const previous = history.past[history.past.length - 1];
  if (!previous) return history;
  return { past: history.past.slice(0, -1), present: previous, future: [history.present, ...history.future] };
}

export function redo(history: History): History {
  const [next, ...rest] = history.future;
  if (!next) return history;
  return { past: [...history.past, history.present], present: next, future: rest };
}
