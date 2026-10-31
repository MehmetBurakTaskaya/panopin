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

  it('ignores the accents of every language, as the index does', () => {
    expect(marked('Café crème à Paris', 'cafe creme', true)).toEqual(['Café', 'crème']);
    expect(marked('Øre Łódź Ελλάδα', 'ore lodz ελλαδα', true)).toEqual(['Øre', 'Łódź', 'Ελλάδα']);
    expect(marked('Ёлка', 'елка', true)).toEqual(['Ёлка']);
    // A vowel mark folds to nothing, and the whole word is marked.
    expect(marked('كَتَبَ الدرس', 'كتب', true)).toEqual(['كَتَبَ']);
    expect(marked('أحمد', 'احمد', true)).toEqual(['أحمد']);
    // Й stays a letter of its own.
    expect(marked('йод', 'иод', true)).toEqual([]);
  });

  it('keeps the text whole', () => {
    const parts = highlight('a İstanbul b', queryTerms('ist', false), false);
    expect(parts.map((p) => p.text).join('')).toBe('a İstanbul b');
    expect(queryTerms('  -- ', false)).toEqual([]);
  });
});
