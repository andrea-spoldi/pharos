import { describe, expect, it } from "vitest";
import cases from "../../../testdata/dot-paths.json";
import { columnsFor, display, field, type Row } from "./fields";

describe("field", () => {
  it("follows dot paths and stops at missing keys", () => {
    const row = { metadata: { name: "pay" }, spec: { tier: 1 } };
    expect(field(row, "metadata.name")).toBe("pay");
    expect(field(row, "spec.tier")).toBe(1);
    expect(field(row, "spec.owner")).toBeUndefined();
    expect(field(row, "metadata.name.x")).toBeUndefined();
  });

  it("indexes into lists", () => {
    expect(field({ tags: ["a", "b"] }, "tags.1")).toBe("b");
  });
});

describe("field shared cases", () => {
  it("field matches shared cases", () => {
    expect(cases.length).toBeGreaterThan(0);
    for (const c of cases as { value: unknown; path: string; found: boolean; expected?: unknown }[]) {
      const got = field(c.value as Row, c.path);
      expect(got !== undefined, `${c.path} on ${JSON.stringify(c.value)}`).toBe(c.found);
      if (c.found) expect(got).toEqual(c.expected);
    }
  });
});

describe("display", () => {
  it("shows scalars as text and structures as compact JSON", () => {
    expect(display("x")).toBe("x");
    expect(display(3)).toBe("3");
    expect(display(false)).toBe("false");
    expect(display(null)).toBe("");
    expect(display(undefined)).toBe("");
    expect(display(["a", 1])).toBe('["a",1]');
  });
});

describe("columnsFor", () => {
  it("uses the declared columns, labelled by field when unlabelled", () => {
    const cols = columnsFor([{ field: "metadata.name", label: "Service" }, { field: "spec.tier" }], []);
    expect(cols).toEqual([
      { field: "metadata.name", label: "Service" },
      { field: "spec.tier", label: "spec.tier" },
    ]);
  });

  it("falls back to the first row's keys, `_source` last", () => {
    const cols = columnsFor(undefined, [{ _source: "a.yaml", name: "a", tier: 1 }]);
    expect(cols.map((c) => c.field)).toEqual(["name", "tier", "_source"]);
  });
});
