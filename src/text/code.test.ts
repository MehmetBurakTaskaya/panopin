import { describe, expect, it } from 'vitest';
import { looksLikeCode } from './code';

describe('looksLikeCode', () => {
  it('knows code, commands, queries, markup and JSON', () => {
    expect(looksLikeCode('fn main() {\n    println!("merhaba");\n}')).toBe(true);
    expect(looksLikeCode('const total = items.reduce((sum, item) => sum + item.price, 0);')).toBe(true);
    expect(looksLikeCode('npm run tauri dev')).toBe(true);
    expect(looksLikeCode('$ git push origin main')).toBe(true);
    expect(looksLikeCode('SELECT * FROM items WHERE id = 1;')).toBe(true);
    expect(looksLikeCode('<div class="note">\n  <p>Merhaba</p>\n</div>')).toBe(true);
    expect(looksLikeCode('{ "name": "Panopin", "version": "1.4.0" }')).toBe(true);
    expect(looksLikeCode('def total(items):\n    return sum(i.price for i in items)\n')).toBe(true);
  });

  it('leaves prose alone', () => {
    expect(looksLikeCode('Toplantı notları: İstanbul ofisi için perşembe 14:00, salon B.')).toBe(false);
    expect(looksLikeCode('The quick brown fox jumps over the lazy dog.')).toBe(false);
    expect(looksLikeCode('Şu adresi kontrol et lütfen, kapıda kod 4512.')).toBe(false);
    expect(looksLikeCode('Import the photos tomorrow;\nthen send them to Ali.')).toBe(false);
    expect(looksLikeCode('let me know when you are free')).toBe(false);
    expect(looksLikeCode('Liste:\n- süt\n- ekmek\n- yumurta')).toBe(false);
    expect(looksLikeCode('')).toBe(false);
  });
});
