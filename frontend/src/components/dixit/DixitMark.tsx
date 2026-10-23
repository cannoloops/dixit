// The Dixit mark: a bold geometric D (Montserrat Bold proportions) whose bottom-left corner runs out
// into a speech-bubble tail. One colour, from `currentColor`; square viewBox, the D in the top 80%.
// Path printed by src-tauri/icons/source/dixit-mark.py (UI); edit the generator, not the path.
const MARK =
  'M11 1.2H47C73.4 1.2 91 17.2 91 41.2C91 65.2 73.4 81.2 47 81.2H43L11 98.8ZM29.4 16.4H47C62.17 16.4 72.28 26.32 72.28 41.2C72.28 56.08 62.17 66 47 66H29.4Z';

export function DixitMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 100 100"
      width="1em"
      height="1em"
      fill="currentColor"
      aria-hidden
      focusable="false"
      className={className}
    >
      <path fillRule="evenodd" d={MARK} />
    </svg>
  );
}
