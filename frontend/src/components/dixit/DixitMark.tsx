// Placeholder of the Dixit mark: replaced by the final SVG (brand assets task). Keep the props.
export function DixitMark({ className }: { className?: string }) {
  return (
    <span aria-hidden className={`incisa inline-flex items-center justify-center ${className ?? ''}`}>
      D·
    </span>
  );
}
