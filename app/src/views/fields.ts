import type { Column, JsonValue } from "../bindings";

/** Mirrors `pharos_engine::MAX_FILES`. */
export const MAX_FILES = 500;

export type Row = { [key in string]: JsonValue };
export type LabelledColumn = { field: string; label: string };

/** The value at a dot path such as `metadata.name`; list items by index. */
export function field(row: Row, path: string): JsonValue | undefined {
  let value: JsonValue | undefined = row;
  for (const key of path.split(".")) {
    if (value === null || typeof value !== "object") return undefined;
    value = Array.isArray(value) ? value[Number(key)] : value[key];
  }
  return value;
}

export function display(value: JsonValue | undefined): string {
  if (value === null || value === undefined) return "";
  return typeof value === "object" ? JSON.stringify(value) : String(value);
}

/** The manifest's columns, or the first row's keys when it declares none. */
export function columnsFor(columns: Column[] | undefined, rows: Row[]): LabelledColumn[] {
  if (columns?.length) return columns.map((c) => ({ field: c.field, label: c.label ?? c.field }));
  const keys = Object.keys(rows[0] ?? {}).filter((k) => k !== "_source");
  if (rows[0] && "_source" in rows[0]) keys.push("_source");
  return keys.map((k) => ({ field: k, label: k }));
}
