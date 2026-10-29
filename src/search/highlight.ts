// Marks the words a search matched, folded the way Rust folds the index (D-017): the four
// Turkish i forms are one letter, case does not count, and optionally Turkish accents neither.

const ACCENTS: Record<string, string> = { ç: 'c', ş: 's', ğ: 'g', ö: 'o', ü: 'u', â: 'a', î: 'i', û: 'u' };

/** One character in, one out, so positions in the folded text are positions in the original. */
export function foldChar(c: string, withoutAccents: boolean): string {
  if (c === 'İ' || c === 'I' || c === 'ı') return 'i';
  const lower = c.toLowerCase();
  const one = [...lower].length === 1 ? lower : c;
  return withoutAccents ? (ACCENTS[one] ?? one) : one;
}

const isWordChar = (c: string) => /[\p{L}\p{N}]/u.test(c);

/** The words of a query, folded; every one matches as the start of a word. */
export function queryTerms(query: string, withoutAccents: boolean): string[] {
  return [...query]
    .map((c) => foldChar(c, withoutAccents))
    .join('')
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean);
}

export interface Segment {
  text: string;
  hit: boolean;
}

/** Splits `text` into plain and matched parts: a match starts a word and begins with a term. */
export function highlight(text: string, terms: string[], withoutAccents: boolean): Segment[] {
  if (terms.length === 0 || !text) return [{ text, hit: false }];
  const chars = [...text];
  const folded = chars.map((c) => foldChar(c, withoutAccents));
  const hit = new Array<boolean>(chars.length).fill(false);
  for (let i = 0; i < chars.length; i++) {
    if (!isWordChar(chars[i]!) || (i > 0 && isWordChar(chars[i - 1]!))) continue;
    for (const term of terms) {
      const length = [...term].length;
      if (folded.slice(i, i + length).join('') === term) hit.fill(true, i, i + length);
    }
  }
  const segments: Segment[] = [];
  for (let i = 0; i < chars.length; i++) {
    const last = segments[segments.length - 1];
    if (last && last.hit === hit[i]) last.text += chars[i];
    else segments.push({ text: chars[i]!, hit: hit[i]! });
  }
  return segments;
}
