/** The Pharos mark (The Beam), from `pharos-brand/pharos-mark.svg`. Inherits `currentColor`. */
export function PharosMark(props: { class?: string }) {
  return (
    <svg class={props.class} viewBox="0 0 64 64" fill="currentColor" aria-hidden="true">
      <path d="M31 38 L14 11 L27 11 Z" />
      <path d="M33 38 L50 11 L37 11 Z" />
      <circle cx="32" cy="40" r="5" />
    </svg>
  );
}
