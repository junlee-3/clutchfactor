/** Last path segment for display — handles both / and \ (Windows). */
export function basename(path: string): string {
  const seg = path.split(/[\\/]/).filter(Boolean);
  return seg[seg.length - 1] ?? path;
}
