import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import type { Column, FileData } from "../bindings";
import { CardView, ListView, TableView } from ".";

const columns: Column[] = [
  { field: "metadata.name", label: "Service" },
  { field: "spec.tier", label: "Tier" },
  { field: "_source", label: "Source" },
];
const data: FileData = {
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

  it("says when files were left out", () => {
    render(() => <TableView columns={columns} data={{ ...data, truncated: true }} />);
    expect(screen.getByText(/Only the first 500 files are shown/)).toBeTruthy();
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
