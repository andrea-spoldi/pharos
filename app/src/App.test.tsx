import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";

vi.mock("./bindings", () => ({
  commands: { ping: vi.fn().mockResolvedValue("pong") },
}));

describe("App", () => {
  it("shows the ping result from the backend", async () => {
    render(() => <App />);
    expect(await screen.findByTestId("ping")).toHaveProperty("textContent", "pong");
  });
});
