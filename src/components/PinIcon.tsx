import { useId } from 'react';

/** The red pushpin from the cork board concept. Decorative. */
export function PinIcon({ className }: { className?: string }) {
  const id = useId();
  return (
    <svg className={className} viewBox="0 0 32 32" aria-hidden="true" focusable="false">
      <defs>
        <radialGradient id={`${id}base`} cx="40%" cy="35%" r="75%">
          <stop offset="0" stopColor="#f0604a" />
          <stop offset=".55" stopColor="#c62a18" />
          <stop offset="1" stopColor="#7e130a" />
        </radialGradient>
        <radialGradient id={`${id}cap`} cx="38%" cy="30%" r="70%">
          <stop offset="0" stopColor="#ffb4a4" />
          <stop offset=".35" stopColor="#ee4a33" />
          <stop offset="1" stopColor="#a51c0e" />
        </radialGradient>
      </defs>
      <circle cx="16" cy="17" r="12" fill={`url(#${id}base)`} />
      <circle cx="16" cy="17" r="11.5" fill="none" stroke="#5e0d06" strokeOpacity=".35" />
      <circle cx="16" cy="14.5" r="7.5" fill={`url(#${id}cap)`} />
      <ellipse cx="13" cy="11" rx="3.2" ry="2" fill="#fff" fillOpacity=".6" />
    </svg>
  );
}
