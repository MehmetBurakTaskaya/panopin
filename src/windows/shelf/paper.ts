// Small pure helpers for how a card looks. Everything derives from the item, so a card keeps its
// angle and colours across openings instead of jumping around.

/** 32-bit FNV-1a over the code points of a string. */
export function fnv1a(text: string): number {
  let hash = 2166136261;
  for (const ch of text) {
    hash ^= ch.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

/** A stable value in [-range, range] for an item id; `salt` separates card, tape angle and offset. */
export function tilt(id: number, salt: string, range: number): number {
  return ((fnv1a(`${id}${salt}`) % 2001) / 1000 - 1) * range;
}

/** Image cards keep the picture's aspect ratio, within limits so a panorama stays readable. */
export function aspectRatio(width: number | null, height: number | null): number {
  if (!width || !height) return 4 / 3;
  return Math.min(2, Math.max(0.56, width / height));
}

// Dark enough for white letters in both themes.
const FAVICON_COLOURS = ['#2f6fb5', '#b4461a', '#2d7d46', '#6d4bc4', '#a83262', '#127066', '#8a6212', '#4b5563'];

/** The letter tile shown until link previews (opt-in) provide a real favicon. */
export function letterTile(domain: string | null): { letter: string; colour: string } {
  const name = domain ?? '';
  // Domains are ASCII or punycode, so no locale casing (no Turkish İ for "instagram").
  const letter = [...name.replace(/^www\./, '')][0]?.toUpperCase() ?? '?';
  return { letter, colour: FAVICON_COLOURS[fnv1a(name) % FAVICON_COLOURS.length]! };
}

/** The part of a link after its domain, for cards without a page title. */
export function linkDetail(url: string | null): string {
  if (!url) return '';
  try {
    const parsed = new URL(url);
    const rest = `${parsed.pathname}${parsed.search}${parsed.hash}`;
    return rest === '/' ? '' : safeDecode(rest);
  } catch {
    return url;
  }
}

function safeDecode(text: string): string {
  try {
    return decodeURI(text);
  } catch {
    return text;
  }
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365 * 86_400_000],
  ['month', 30 * 86_400_000],
  ['week', 7 * 86_400_000],
  ['day', 86_400_000],
  ['hour', 3_600_000],
  ['minute', 60_000],
];

/** "5 min. ago" style age of the last copy, in the UI language. */
export function age(then: number, now: number, lang: string): string {
  const format = new Intl.RelativeTimeFormat(lang, { numeric: 'auto', style: 'short' });
  const elapsed = Math.max(0, now - then);
  for (const [unit, ms] of UNITS) {
    if (elapsed >= ms) return format.format(-Math.floor(elapsed / ms), unit);
  }
  return format.format(0, 'second');
}

/** What a link card is called: the user's name, else the page title, else the domain. */
export function linkHeading(item: { title: string | null; linkTitle: string | null; domain: string | null; url: string | null }): string {
  return item.title ?? item.linkTitle ?? item.domain ?? item.url ?? '';
}

/** A YouTube video link, whose picture gets a play mark. */
export function isVideoLink(url: string | null): boolean {
  if (!url) return false;
  try {
    const { hostname, pathname, searchParams } = new URL(url);
    const host = hostname.replace(/^www\./, '');
    if (host === 'youtu.be') return pathname.length > 1;
    return /^(m\.|music\.)?youtube\.com$/.test(host) && (searchParams.has('v') || pathname.startsWith('/shorts/'));
  } catch {
    return false;
  }
}

/** A recording's length as m:ss or h:mm:ss. */
export function duration(ms: number | null): string {
  if (ms === null || !Number.isFinite(ms)) return '';
  const total = Math.round(ms / 1000);
  const [h, m, s] = [Math.floor(total / 3600), Math.floor((total % 3600) / 60), total % 60];
  const pad = (n: number) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}
