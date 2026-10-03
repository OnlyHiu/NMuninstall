/** Minimal `className` joiner — no dependency needed for a fixed set of shapes. */
export function cn(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(' ');
}
