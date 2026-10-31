// Marks the words a search matched, folded the way Rust folds the index (D-017): the four
// Turkish i forms are one letter, case does not count, and optionally accents neither (D-082).

// Letters with a stroke or a ring that no decomposition takes apart.
const PLAIN: Record<string, string> = { ø: 'o', ł: 'l', đ: 'd', ð: 'd', ħ: 'h', ё: 'е' };
// Accents come off in Latin, Greek and Arabic only; й, kana with dakuten and Hangul stay.
const BARE_SCRIPTS = /[À-ɏḀ-ỿͰ-Ͽἀ-῿ؠ-ۿ]/u;
// Latin and Greek accents and Arabic vowel marks written after their letter.
const LOOSE_MARK = /[̀-ًͯ-ٰٟ]/u;

function bareChar(c: string): string {
  const plain = PLAIN[c];
  if (plain) return plain;
  if (LOOSE_MARK.test(c)) return '';
  if (!BARE_SCRIPTS.test(c)) return c;
  const [base, ...marks] = [...c.normalize('NFD')];
  return base && marks.every((mark) => /\p{M}/u.test(mark)) ? base : c;
}

/** One character in, at most one out, so positions in the folded text follow the original's. */
export function foldChar(c: string, withoutAccents: boolean): string {
  if (c === 'İ' || c === 'I' || c === 'ı') return 'i';
  const lower = c.toLowerCase();
  const one = [...lower].length === 1 ? lower : c;
  return withoutAccents ? bareChar(one) : one;
}

// Marks belong to their word: an Arabic vowel mark does not start a new one.
const isWordChar = (c: string) => /[\p{L}\p{N}\p{M}]/u.test(c);

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
      // A dropped vowel mark folds to nothing, so the match may span more characters.
      let found = '';
      let end = i;
      while (end < chars.length && found.length < term.length) found += folded[end++];
      if (found !== term) continue;
      // The marks on the last letter belong to the match too.
      while (end < chars.length && folded[end] === '') end++;
      hit.fill(true, i, end);
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
