// The platform a window runs on: key names and some texts differ between Windows and macOS.
export type Platform = 'windows' | 'mac';

export function detectPlatform(userAgent: string = navigator.userAgent): Platform {
  return /Mac OS X|Macintosh/.test(userAgent) ? 'mac' : 'windows';
}
