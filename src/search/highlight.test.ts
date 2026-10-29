import { describe, expect, it } from 'vitest';
import { highlight, queryTerms } from './highlight';

const marked = (text: string, query: string, withoutAccents = false) =>
  highlight(text, queryTerms(query, withoutAccents), withoutAccents)
    .filter((part) => part.hit)
    .map((part) => part.text);

describe('search highlighting', () => {
  it('marks word starts with Turkish-aware folding', () => {
    expect(marked('Toplantı İstanbul ofisinde', 'istan')).toEqual(['İstan']);
    expect(marked('IŞIK AÇIK', 'ışık')).toEqual(['IŞIK']);
    expect(marked('kalemlik ve kalem', 'kal')).toEqual(['kal', 'kal']);
    // Only at the start of a word.
    expect(marked('sokak', 'ok')).toEqual([]);
  });

  it('ignores accents only when asked', () => {
    expect(marked('Bahçede çiçek', 'cicek')).toEqual([]);
    expect(marked('Bahçede çiçek', 'cicek', true)).toEqual(['çiçek']);
  });

  it('keeps the text whole', () => {
    const parts = highlight('a İstanbul b', queryTerms('ist', false), false);
    expect(parts.map((p) => p.text).join('')).toBe('a İstanbul b');
    expect(queryTerms('  -- ', false)).toEqual([]);
  });
});
