import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import type { Column, WidgetData } from "../bindings";
import { CardView, KvView, ListView, looksMachineMade, RawView, TableView } from ".";

const columns: Column[] = [
  { field: "metadata.name", label: "Service" },
  { field: "spec.tier", label: "Tier" },
  { field: "_source", label: "Source" },
];
const data: WidgetData = {
  rows: [
    { metadata: { name: "notification-service" }, spec: { tier: 2 }, _source: "catalog/services/n.yaml" },
    { metadata: { name: "payment-service" }, spec: { tier: 1 }, _source: "catalog/services/p.yaml" },
  ],
  truncated: false,
};

describe("TableView", () => {
  it("renders one header per column and one row per item", () => {
    render(() => <TableView columns={columns} data={data} />);
    const headers = screen.getAllByRole("columnheader").map((h) => h.textContent);
    expect(headers).toEqual(["Service", "Tier", "Source"]);
    const rows = screen.getAllByRole("row").slice(1);
    expect(rows.map((r) => [...r.querySelectorAll("td")].map((c) => c.textContent))).toEqual([
      ["notification-service", "2", "catalog/services/n.yaml"],
      ["payment-service", "1", "catalog/services/p.yaml"],
    ]);
  });

  it("says when rows were left out", () => {
    render(() => <TableView columns={columns} data={{ ...data, truncated: true }} />);
    expect(screen.getByText("Only the first 500 rows are shown.")).toBeTruthy();
  });
});

describe("ListView", () => {
  it("leads each item with the first column and lists the rest after it", () => {
    render(() => <ListView columns={columns} data={data} />);
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(2);
    expect(items[1].querySelector("strong")?.textContent).toBe("payment-service");
    expect(items[1].textContent).toContain("Tier: 1");
    expect(items[1].textContent).toContain("Source: catalog/services/p.yaml");
  });
});

describe("CardView", () => {
  it("titles each card with the first column and labels the other fields", () => {
    render(() => <CardView columns={columns} data={data} />);
    const cards = screen.getAllByRole("article");
    expect(cards).toHaveLength(2);
    expect(cards[0].querySelector("h3")?.textContent).toBe("notification-service");
    const terms = [...cards[0].querySelectorAll("dt")].map((t) => t.textContent);
    const values = [...cards[0].querySelectorAll("dd")].map((d) => d.textContent);
    expect(terms).toEqual(["Tier", "Source"]);
    expect(values).toEqual(["2", "catalog/services/n.yaml"]);
  });
});

describe("RawView", () => {
  it("shows plain command output as is", () => {
    render(() => <RawView data={{ rows: [{ value: "kind-dev" }], truncated: false }} />);
    expect(document.querySelector("pre.raw")?.textContent).toBe("kind-dev");
  });

  it("pretty-prints structured rows", () => {
    render(() => <RawView data={{ rows: [{ name: "a" }, { name: "b" }], truncated: false }} />);
    expect(document.querySelector("pre.raw")?.textContent).toBe(JSON.stringify([{ name: "a" }, { name: "b" }], null, 2));
  });
});

describe("KvView", () => {
  const one = (row: Record<string, unknown>, cols?: Column[]) =>
    render(() => <KvView columns={cols} data={{ rows: [row as WidgetData["rows"][number]], truncated: false }} />);
  const terms = () => [...document.querySelectorAll("dt")].map((t) => t.textContent);
  const values = () => [...document.querySelectorAll("dd")];

  it("renders_columns_in_order_with_labels", () => {
    one({ Arn: "x", Account: "1" }, [{ field: "Account", label: "Account" }, { field: "Arn", label: null }]);
    expect(terms()).toEqual(["Account", "Arn"]);
  });

  it("without_columns_shows_top_level_keys", () => {
    one({ b: 1, a: 2 });
    expect(terms()).toEqual(["b", "a"]);
  });

  it("source_hidden_unless_listed", () => {
    one({ a: 1, _source: "s.yaml" });
    expect(terms()).toEqual(["a"]);
    document.body.innerHTML = "";
    one({ a: 1, _source: "s.yaml" }, [{ field: "_source", label: "Source" }]);
    expect(terms()).toEqual(["Source"]);
  });

  it("nested_values_render_as_compact_json", () => {
    one({ context: { cluster: "c", namespace: "n" } });
    expect(values()[0].textContent).toBe('{"cluster":"c","namespace":"n"}');
    expect(values()[0].classList.contains("mono")).toBe(true);
  });

  it("machine_values_are_mono", () => {
    one({ arn: "arn:aws:iam::1:user/x", url: "https://x", env: "dev" });
    expect(values().map((v) => v.classList.contains("mono"))).toEqual([true, true, false]);
    expect(looksMachineMade("123e4567-e89b-12d3-a456-426614174000")).toBe(true);
    expect(looksMachineMade("0123456789abcdef")).toBe(true);
    expect(looksMachineMade("dev")).toBe(false);
  });

  it("more_rows_note", () => {
    render(() => <KvView data={{ rows: [{ a: 1 }, { a: 2 }, { a: 3 }], truncated: false }} />);
    expect(screen.getByText("+ 2 more rows — use a table view")).toBeTruthy();
    expect(document.querySelectorAll("dd")).toHaveLength(1);
  });

  it("missing_column_shows_empty_value", () => {
    one({ a: 1 }, [{ field: "nope", label: "Nope" }]);
    expect(terms()).toEqual(["Nope"]);
    expect(values()[0].textContent).toBe("");
  });

  it("long_values_wrap", () => {
    one({ a: "x".repeat(200) });
    expect(values()[0].classList.contains("kv-value")).toBe(true);
  });
});
