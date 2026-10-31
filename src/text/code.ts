// Whether a copied text looks like code, so cards can show it in a monospace font (D-083). It
// only changes the look, so it leans towards prose: one weak sign is not enough.

// Words that start a line of code. Case counts: prose may begin with "Import" or "Let".
const KEYWORD =
  /^\s*(?:(?:fn|function|const|let|var|def|class|import|export|return|package|public|private|using|namespace|struct|impl|interface)\s|(?:SELECT|INSERT|UPDATE|DELETE|CREATE|ALTER|WITH)\s|#include|#!|<\?php|<!DOCTYPE|<!doctype|@echo)/;
// A command line: a common tool followed by its arguments.
const COMMAND =
  /^\s*(?:\$\s+)?(?:npm|npx|pnpm|yarn|node|git|gh|cargo|rustup|python3?|pip3?|cd|ls|mkdir|rm|cp|mv|curl|wget|docker|kubectl|sudo|brew|winget|choco|ssh|scp|pwsh|powershell)\s+\S/;
// Punctuation that prose rarely has: a line ending in ; { } or "):", arrows, == and closing tags.
const SYMBOLS = /[;{}]\s*$|\):\s*$|=>|::|->|===?|!==?|&&|\|\||<\/\w|\/>|\(\)/;

export function looksLikeCode(text: string): boolean {
  const lines = text
    .slice(0, 800)
    .split(/\r?\n/)
    .filter((line) => line.trim());
  const first = lines[0];
  if (!first) return false;
  const whole = text.trim();
  // JSON: braces or brackets around "key": value pairs.
  if (/^[[{][\s\S]*[\]}]$/.test(whole) && /"[^"\n]*"\s*:/.test(whole)) return true;
  if (lines.length === 1) return COMMAND.test(first) || (KEYWORD.test(first) && SYMBOLS.test(first));
  const keywords = lines.filter((line) => KEYWORD.test(line)).length;
  const symbols = lines.filter((line) => SYMBOLS.test(line)).length;
  // Half the lines show code, by a keyword and punctuation or by punctuation on two lines.
  return symbols > 0 && (keywords > 0 || symbols > 1) && keywords + symbols >= Math.ceil(lines.length / 2);
}
