import { describe, expect, it } from "vitest";
import { resolveLayout, type LayoutItem } from "./grid-layout";

const full = (id: number, y: number, h = 2): LayoutItem => ({ id, x: 0, y, w: 12, h });

describe("resolveLayout", () => {
  it("moving_a_card_pins_it_at_the_clamped_target", () => {
    const items: LayoutItem[] = [
      { id: 1, x: 0, y: 0, w: 4, h: 2 },
      { id: 2, x: 4, y: 0, w: 4, h: 2 },
    ];
    const out = resolveLayout(items, 1, { x: 6, y: 3, w: 4, h: 2 });
    expect(out.find((i) => i.id === 1)).toEqual({ id: 1, x: 6, y: 3, w: 4, h: 2 });
  });

  it("dropping_onto_another_pushes_it_and_compacts", () => {
    const out = resolveLayout([full(1, 0), full(2, 2)], 2, { x: 0, y: 0, w: 12, h: 2 });
    expect(out).toEqual([full(1, 2), full(2, 0)]);
  });

  it("resize_grow_pushes_the_card_below_down", () => {
    const out = resolveLayout([full(1, 0), full(2, 2)], 1, { x: 0, y: 0, w: 12, h: 3 });
    expect(out).toEqual([full(1, 0, 3), full(2, 3)]);
  });

  it("target_x_is_clamped_to_grid_width", () => {
    const out = resolveLayout([{ id: 1, x: 0, y: 0, w: 4, h: 2 }], 1, { x: 10, y: 0, w: 4, h: 2 });
    expect(out[0].x).toBe(8);
  });

  it("no_op_target_returns_equivalent_geometry", () => {
    const items: LayoutItem[] = [
      { id: 1, x: 0, y: 0, w: 6, h: 2 },
      { id: 2, x: 6, y: 0, w: 6, h: 3 },
      { id: 3, x: 0, y: 3, w: 12, h: 2 },
    ];
    const out = resolveLayout(items, 2, { x: 6, y: 0, w: 6, h: 3 });
    expect(out).toEqual(items);
  });
});
