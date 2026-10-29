// Small line icons for the preview toolbars (20×20, drawn with the current text colour).
import type { ReactNode } from 'react';

function Icon({ children, filled = false }: { children: ReactNode; filled?: boolean }) {
  return (
    <svg
      viewBox="0 0 20 20"
      aria-hidden="true"
      focusable="false"
      fill={filled ? 'currentColor' : 'none'}
      stroke={filled ? 'none' : 'currentColor'}
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {children}
    </svg>
  );
}

export const icons = {
  select: (
    <Icon filled>
      <path d="M5 2.5l10.5 7-4.6 1.1 2.6 5-2.1 1.1-2.6-5-3.8 3z" />
    </Icon>
  ),
  pen: (
    <Icon>
      <path d="M3.5 16.5l1-4 8.7-8.7a1.7 1.7 0 0 1 2.4 2.4l-8.7 8.7z" />
    </Icon>
  ),
  highlighter: (
    <Icon>
      <path d="M6 12.5l6.6-6.6 2.6 2.6-6.6 6.6H6z" />
      <path d="M3 17.5h14" />
    </Icon>
  ),
  rect: (
    <Icon>
      <rect x="3" y="5" width="14" height="10" rx="1" />
    </Icon>
  ),
  ellipse: (
    <Icon>
      <ellipse cx="10" cy="10" rx="7.5" ry="5.5" />
    </Icon>
  ),
  arrow: (
    <Icon>
      <path d="M4 16L15.5 4.5M9.5 4.5h6v6" />
    </Icon>
  ),
  line: (
    <Icon>
      <path d="M4 16L16 4" />
    </Icon>
  ),
  text: (
    <Icon>
      <path d="M4.5 5h11M10 5v11M7.5 16h5" />
    </Icon>
  ),
  pixelate: (
    <Icon filled>
      <path d="M3 3h4v4H3zM10 3h3v4h-3zM14 7h3v3h-3zM3 10h4v3H3zM8 8h4v4H8zM10 14h4v3h-4zM3 14h4v3H3z" />
    </Icon>
  ),
  undo: (
    <Icon>
      <path d="M7.5 5.5H4V2" />
      <path d="M4.3 5.5A6.5 6.5 0 1 1 3.5 11" />
    </Icon>
  ),
  redo: (
    <Icon>
      <path d="M12.5 5.5H16V2" />
      <path d="M15.7 5.5A6.5 6.5 0 1 0 16.5 11" />
    </Icon>
  ),
  crop: (
    <Icon>
      <path d="M6 2v12h12M2 6h12v12" />
    </Icon>
  ),
  revert: (
    <Icon>
      <path d="M3.5 3.5v4h4" />
      <path d="M3.9 7.5A6.5 6.5 0 1 1 3.6 12" />
      <path d="M10 7v3.5l2.3 1.4" />
    </Icon>
  ),
  check: (
    <Icon>
      <path d="M4 10.5l4 4 8-9" />
    </Icon>
  ),
  close: (
    <Icon>
      <path d="M5 5l10 10M15 5L5 15" />
    </Icon>
  ),
};
