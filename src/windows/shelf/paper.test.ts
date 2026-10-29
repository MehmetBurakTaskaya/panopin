import { describe, expect, it } from 'vitest';
import { age, aspectRatio, fnv1a, letterTile, linkDetail, tilt } from './paper';

describe('card paper', () => {
  it('derives a stable tilt within range from the id', () => {
    expect(fnv1a('')).toBe(2166136261);
    for (let id = 1; id < 200; id++) {
      const angle = tilt(id, 'r', 3);
      expect(Math.abs(angle)).toBeLessThanOrEqual(3);
      expect(tilt(id, 'r', 3)).toBe(angle);
    }
    expect(tilt(5, 'r', 3)).not.toBe(tilt(5, 't', 3));
  });

  it('keeps aspect ratios readable', () => {
    expect(aspectRatio(1920, 1080)).toBeCloseTo(16 / 9);
    expect(aspectRatio(5000, 100)).toBe(2);
    expect(aspectRatio(100, 5000)).toBe(0.56);
    expect(aspectRatio(null, 10)).toBeCloseTo(4 / 3);
  });

  it('builds letter tiles and link details', () => {
    expect(letterTile('github.com').letter).toBe('G');
    expect(letterTile('istanbul.example').letter).toBe('I');
    expect(letterTile('github.com').colour).toBe(letterTile('github.com').colour);
    expect(linkDetail('https://example.com/')).toBe('');
    expect(linkDetail('https://example.com/a%C3%A7%C4%B1k?q=1')).toBe('/açık?q=1');
    expect(linkDetail('not a url')).toBe('not a url');
  });

  it('formats ages in the UI language', () => {
    const now = Date.UTC(2026, 9, 29, 12);
    expect(age(now - 5 * 60_000, now, 'en')).toMatch(/5/);
    expect(age(now - 2 * 3_600_000, now, 'tr')).toMatch(/2/);
    expect(age(now + 10_000, now, 'en')).toBeTruthy();
  });
});
