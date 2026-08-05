import type { SVGProps } from 'react';

export function PastedMark(props: SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 32 32" fill="none" aria-hidden="true" {...props}>
      <g stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <path d="M21 8h2.5A3.5 3.5 0 0 1 27 11.5v14A3.5 3.5 0 0 1 23.5 29h-14A3.5 3.5 0 0 1 6 25.5V23" opacity=".72" />
        <path d="M8.5 6h15A2.5 2.5 0 0 1 26 8.5v15a2.5 2.5 0 0 1-2.5 2.5h-15A2.5 2.5 0 0 1 6 23.5v-15A2.5 2.5 0 0 1 8.5 6Z" />
        <path d="M12 6V4.75A1.75 1.75 0 0 1 13.75 3h4.5A1.75 1.75 0 0 1 20 4.75V6" />
        <path d="M11 14h10M11 19h7" />
      </g>
    </svg>
  );
}
