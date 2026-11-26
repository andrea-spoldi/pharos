export type Box = { x: number; y: number; w: number; h: number };
export type LayoutItem = Box & { id: number };

export type DragMode = "move" | "resize";

/**
 * The move/resize coordinator a dashboard hands to its top-level frames. The pixel→cell
 * math lives in the frame's pointer handlers; everything past a resolved target `Box` lives
 * here, so a test can drive a whole gesture without synthesising pointer physics.
 */
export type DragController = {
  /** Affordances are live: edit mode, an editable dashboard, wider than the single-column breakpoint. */
  enabled: () => boolean;
  /** A gesture is in flight. */
  active: () => boolean;
  /** The placement currently being dragged, if any. */
  isMoving: (id: number) => boolean;
  /** Start a gesture, snapshotting the current layout as its base. */
  begin: (id: number, mode: DragMode) => void;
  /** Preview a target box: clamp, reflow, hold the result as the transient layout. */
  onMove: (id: number, target: Box) => void;
  /** Finish: persist the placements whose geometry changed, or nothing if none did. */
  commit: (id: number) => void;
  /** Abort: drop the transient layout, persist nothing. */
  cancel: () => void;
  /** The transient box for a placement while a gesture is in flight, else undefined. */
  boxOf: (id: number) => Box | undefined;
};

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
