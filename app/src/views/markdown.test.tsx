import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { MarkdownView } from "./markdown";

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
