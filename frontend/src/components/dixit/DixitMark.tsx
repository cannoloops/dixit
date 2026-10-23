// The Dixit mark: a Roman capital D with a triangular interpunct (D·), V-cut.
// One colour, from `currentColor` (rail: text-rail-accent, onboarding: text-primary). The facets are
// the same colour at three strengths, so the cut reads as incised on dark stone and as a bronze
// prism on marble. Generated from src-tauri/icons/source/dixit-mark.py; edit the generator, not the paths.
const SILHOUETTE =
  'M0 0L46 0C78.40 0 100 20 100 50C100 80 78.40 100 46 100L0 100L0 98.20C7 98.20 10 92.20 10 83.20L10 16.80C10 7.80 7 1.80 0 1.80L0 0ZM33 18.50L33 81.50C33 83.43 34.57 85 36.50 85L44.20 85C62.06 85 75 70.94 75 48C75 27.54 62.06 15 44.20 15L36.50 15C34.57 15 33 16.57 33 18.50ZM109 44.52L128 44.52L118.50 60.97L109 44.52Z';
const MID =
  'M77.3 7.9L81.7 10.9L86.2 14.8L90.1 19.2L93.1 23.5L83.2 29.4L81.0 25.6L78.3 21.9L75.6 19.0L72.4 16.2ZM7.6 5.9L6.4 4.3L4.8 3.0L5.7 1.6L8.2 2.5L10.9 3.9ZM33.4 83.0L33.8 83.7L35.0 84.6L31.4 91.9L28.8 91.1L25.9 90.0L22.8 88.2ZM70.0 28.6L68.2 26.1L65.8 23.3L63.6 21.3L60.6 19.3L65.0 11.8L68.5 13.6L72.6 16.4L76.4 19.8L79.1 22.9Z';
const LIT =
  'M93.1 23.5L95.2 27.5L96.9 31.7L98.3 36.1L99.2 40.0L99.8 44.9L100.0 49.1L99.9 54.3L99.3 59.2L98.5 63.1L97.2 67.6L95.8 71.2L93.8 75.2L91.9 78.4L89.2 81.9L86.2 85.2L82.9 88.2L79.9 90.4L76.7 92.5L70.4 95.6L63.4 97.9L55.9 99.3L51.5 99.8L46.9 100.0L0.0 100.0L0.2 99.7L0.8 99.2L4.0 98.8L6.3 98.1L10.3 96.4L15.1 93.5L21.4 88.3L22.2 87.9L25.4 89.7L27.8 90.8L30.9 91.8L33.3 92.3L36.5 92.5L44.4 92.5L50.1 92.2L56.3 91.1L59.2 90.3L62.6 89.1L68.0 86.4L70.7 84.6L73.1 82.7L75.8 80.3L77.7 78.2L79.8 75.5L81.8 72.3L83.3 69.4L84.6 66.3L85.7 63.1L86.6 59.0L87.2 55.3L87.5 51.4L87.5 47.3L87.2 43.4L86.6 39.7L85.8 36.3L84.7 33.0L83.2 29.4ZM4.8 3.0L2.3 2.0L0.0 1.8L0.2 1.5L0.9 0.9L1.7 1.0L5.7 1.6ZM33.0 18.5L33.0 81.5L33.4 83.0L22.8 88.2L21.7 84.9L21.5 82.2L21.5 17.8ZM60.6 19.3L56.5 17.3L52.9 16.1L49.1 15.3L45.0 15.0L35.7 15.1L35.0 15.4L33.8 16.3L33.1 17.7L33.0 18.5L21.5 17.8L21.7 15.1L22.8 11.8L25.9 10.0L28.8 8.9L31.4 8.1L34.0 7.6L36.1 7.5L46.2 7.5L52.1 8.0L55.9 8.7L58.7 9.4L62.3 10.6L65.0 11.8ZM128.0 44.5L118.5 61.0L118.2 60.5L118.5 60.4L118.5 50.0L127.8 44.7Z';

export function DixitMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 128 100"
      width="1.28em"
      height="1em"
      fill="currentColor"
      aria-hidden
      focusable="false"
      className={className}
    >
      <path fillRule="evenodd" fillOpacity={0.58} d={SILHOUETTE} />
      <path fillOpacity={0.79} d={MID} />
      <path d={LIT} />
    </svg>
  );
}
