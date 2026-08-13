import type { SVGProps } from 'react';

/** A small resident copycat, not a replacement for the Pasted product mark. */
export function CopycatMark(props: SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 48 48" fill="none" aria-hidden="true" {...props}>
      <g stroke="currentColor" strokeWidth="2.35" strokeLinecap="round" strokeLinejoin="round">
        <path d="M31.5 11H35a5 5 0 0 1 5 5v23a5 5 0 0 1-5 5H13a5 5 0 0 1-5-5v-4" opacity=".45" />
        <path d="M12 9h24a4 4 0 0 1 4 4v22a4 4 0 0 1-4 4H12a4 4 0 0 1-4-4V13a4 4 0 0 1 4-4Z" />
        <path d="m15 20 2-7 6 4h2l6-4 2 7v3.5c0 5-4 9-9 9s-9-4-9-9V20Z" />
        <path d="M20.25 23.25h.01M27.75 23.25h.01M22 27c1.2 1 2.8 1 4 0M15.5 26l-4 1M32.5 26l4 1M15.75 29l-3.5 2M32.25 29l3.5 2" />
      </g>
    </svg>
  );
}
