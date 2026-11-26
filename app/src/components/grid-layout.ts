export type Box = { x: number; y: number; w: number; h: number };
export type LayoutItem = Box & { id: number };

const COLS = 12;

function collides(a: Box, b: Box): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/**
 * Pin `movedId` at `target` (x clamped to the grid), then compact every other
 * item vertically in (y, x) order. Returns items in input order.
 */
export function resolveLayout(items: LayoutItem[], movedId: number, target: Box): LayoutItem[] {
  const x = Math.max(0, Math.min(target.x, COLS - target.w));
  const pinned: LayoutItem = { id: movedId, x, y: target.y, w: target.w, h: target.h };
  const placed: LayoutItem[] = [pinned];
  const result = new Map<number, LayoutItem>([[movedId, pinned]]);

  const others = items
    .filter((i) => i.id !== movedId)
    .sort((a, b) => a.y - b.y || a.x - b.x);
  for (const item of others) {
    let y = 0;
    while (placed.some((p) => collides({ ...item, y }, p))) y++;
    const next = { ...item, y };
    placed.push(next);
    result.set(item.id, next);
  }
  return items.map((i) => result.get(i.id) as LayoutItem);
}
