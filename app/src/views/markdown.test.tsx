import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import type { WidgetData } from "../bindings";
import { capText, MarkdownSourceView, MarkdownView, MAX_MARKDOWN_BYTES } from "./markdown";

const data = (...rows: WidgetData["rows"]): WidgetData => ({ rows, truncated: false });

describe("MarkdownView", () => {
  it("renders headings two levels down and paragraphs", () => {
    const { container } = render(() => <MarkdownView content={"# Hi\n\ntext"} />);
    expect(container.querySelector("h3")?.textContent).toBe("Hi");
    expect(container.querySelector("p")?.textContent).toBe("text");
  });

  it("shows raw HTML as text", () => {
    const { container } = render(() => <MarkdownView content={"<b>x</b>"} />);
    expect(screen.getByText("<b>x</b>")).toBeTruthy();
    expect(container.querySelector("b")).toBeNull();
  });

  it("shows links as text with their URL", () => {
    const { container } = render(() => <MarkdownView content={"[docs](https://d.example)"} />);
    expect(screen.getByText("docs")).toBeTruthy();
    expect(screen.getByText("(https://d.example)")).toBeTruthy();
    expect(container.querySelector("a")).toBeNull();
  });

  it("shows images as their alt text", () => {
    const { container } = render(() => <MarkdownView content={"![alt](https://i.example/x.png)"} />);
    expect(screen.getByText("alt")).toBeTruthy();
    expect(container.querySelector("img")).toBeNull();
  });
});

describe("MarkdownSourceView", () => {
  it("renders_from_value", () => {
    const { container } = render(() => <MarkdownSourceView data={data({ value: "# Hi\n\nText" })} />);
    expect(container.querySelector("h3")?.textContent).toBe("Hi");
  });

  it("renders_from_named_column", () => {
    const { container } = render(() => (
      <MarkdownSourceView columns={[{ field: "body", label: null }]} data={data({ body: "# Named", value: "# Other" })} />
    ));
    expect(container.querySelector("h3")?.textContent).toBe("Named");
  });

  it("non_string_value_as_json", () => {
    const { container } = render(() => <MarkdownSourceView data={data({ value: { a: 1 } })} />);
    expect(container.textContent).toContain('{"a":1}');
  });

  it("cap_note_when_over_256kb", () => {
    const { container } = render(() => <MarkdownSourceView data={data({ value: "a".repeat(MAX_MARKDOWN_BYTES + 10) })} />);
    expect(screen.getByText("Only the first 256 KB is shown.")).toBeTruthy();
    expect(container.querySelector(".markdown")?.textContent?.length).toBe(MAX_MARKDOWN_BYTES);
  });

  it("no_cap_note_when_small", () => {
    render(() => <MarkdownSourceView data={data({ value: "x" })} />);
    expect(screen.queryByText(/Only the first/)).toBeNull();
  });

  it("cap_does_not_split_characters", () => {
    const s = "€".repeat(10); // 3 bytes each
    const r = capText(s, 10);
    expect(r.cut).toBe(true);
    expect(r.text).toBe("€€€");
    expect(new TextEncoder().encode(r.text).length).toBeLessThanOrEqual(10);
    expect(capText("abc", 10)).toEqual({ text: "abc", cut: false });
  });
});
