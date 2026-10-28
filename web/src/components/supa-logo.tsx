/** Exact geometry of the existing PixelMark; the master logo stays sharp. */
export function SupaLogo({ size = 24, className }: { size?: number; className?: string }) {
  return <svg className={className} data-supa-logo="true" viewBox="0 0 24 24" width={size} height={size} fill="#71957d" shapeRendering="crispEdges" aria-hidden="true" focusable="false" style={{ flexShrink: 0, display: "block" }}><path fillRule="evenodd" d="M7 2h10v2h4v4h2v10h-4v4H5v-4H1V8h2V4h4zM6 9h4v5H6zm8 0h4v5h-4zM8 17h8v2H8z"/></svg>;
}
