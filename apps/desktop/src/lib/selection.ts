export function moveSelection(index: number, direction: -1 | 1, count: number): number {
  if (count <= 0) return -1;
  if (index < 0) return direction === 1 ? 0 : count - 1;
  return Math.max(0, Math.min(count - 1, index + direction));
}
export function selectedIndex(index: number, count: number): number {
  return count === 0 ? -1 : Math.max(0, Math.min(count - 1, index));
}
