// ContextPage tests — the Workspace Context graph page renders the
// workspace as a live hub-and-spoke graph: App.tsx at the center, active
// context around it, wider workspace artifacts beyond. Selection and
// search drive the emphasis model; the App.tsx inspector stays pinned.

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { ContextPage } from "./ContextPage";

describe("ContextPage", () => {
  afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks(); });

  it("renders the header, subtitle and search field", () => {
    render(<ContextPage />);
    expect(screen.getByText("Workspace Context")).toBeInTheDocument();
    expect(screen.getByText("Relationships discovered across your workspace")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Search context")).toBeInTheDocument();
  });

  it("renders the full workspace graph — entry, active context, and distant artifacts", () => {
    render(<ContextPage />);

    // Foreground — the entry point.
    expect(screen.getByRole("button", { name: "App.tsx · Workspace Entry Point" })).toBeInTheDocument();

    // Active context ring.
    expect(screen.getByRole("button", { name: "components/Header.tsx · Component" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "components/Sidebar.tsx · Component" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "components/Dashboard.tsx · Component" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "services/api.ts · Service" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "services/auth.ts · Service" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "hooks/useWorkspace.ts · Hook" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "styles/theme.css · Styles" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "tests/App.test.tsx · Test" })).toBeInTheDocument();

    // Wider workspace context.
    expect(screen.getByRole("button", { name: "README.md · Documentation" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "docs/architecture.md · Documentation" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "package.json · Configuration" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "documentation/ · Folder" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "repositories/ · Folder" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "screenshots/ · Folder" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "tests/ · Folder" })).toBeInTheDocument();
  });

  it("keeps the App.tsx inspector floating, subtle and secondary", () => {
    render(<ContextPage />);
    expect(screen.getAllByText("App.tsx").length).toBeGreaterThan(0);
    expect(screen.getAllByText("React Component").length).toBeGreaterThan(0);
    expect(screen.getByText("Connections")).toBeInTheDocument();
    expect(screen.getByText("8")).toBeInTheDocument();
    expect(screen.getAllByText("Workspace Entry Point").length).toBeGreaterThan(0);
    expect(screen.getByText("Active")).toBeInTheDocument();
  });

  it("emphasizes a node's relationships on selection and de-emphasizes the rest", () => {
    render(<ContextPage />);

    const header = screen.getByRole("button", { name: "components/Header.tsx · Component" });
    fireEvent.pointerDown(header, { pointerId: 1, button: 0, clientX: 10, clientY: 10 });
    fireEvent.pointerUp(header, { pointerId: 1, button: 0, clientX: 10, clientY: 10 });

    expect(header).toHaveAttribute("data-emphasis", "focus");
    expect(screen.getByRole("button", { name: "App.tsx · Workspace Entry Point" })).toHaveAttribute(
      "data-emphasis",
      "related",
    );
    expect(screen.getByRole("button", { name: "services/auth.ts · Service" })).toHaveAttribute(
      "data-emphasis",
      "dimmed",
    );
    expect(header).toHaveAttribute("data-selected");
  });

  it("hovering a node emphasizes it and its relationships live", () => {
    render(<ContextPage />);

    const api = screen.getByRole("button", { name: "services/api.ts · Service" });
    fireEvent.pointerEnter(api);

    expect(api).toHaveAttribute("data-emphasis", "focus");
    expect(screen.getByRole("button", { name: "App.tsx · Workspace Entry Point" })).toHaveAttribute(
      "data-emphasis",
      "related",
    );
    expect(screen.getByRole("button", { name: "styles/theme.css · Styles" })).toHaveAttribute(
      "data-emphasis",
      "dimmed",
    );

    fireEvent.pointerLeave(api);
    expect(api).toHaveAttribute("data-emphasis", "default");
  });

  it("search dims non-matching context and Enter focuses the first match", () => {
    render(<ContextPage />);

    const input = screen.getByPlaceholderText("Search context");
    fireEvent.change(input, { target: { value: "api" } });

    expect(screen.getByRole("button", { name: "services/api.ts · Service" })).toHaveAttribute(
      "data-emphasis",
      "match",
    );
    expect(screen.getByRole("button", { name: "components/Header.tsx · Component" })).toHaveAttribute(
      "data-emphasis",
      "dimmed",
    );

    fireEvent.keyDown(input, { key: "Enter" });
    expect(screen.getByRole("button", { name: "services/api.ts · Service" })).toHaveAttribute(
      "data-selected",
    );
  });

  it("clears search with Escape", () => {
    render(<ContextPage />);

    const input = screen.getByPlaceholderText("Search context");
    fireEvent.change(input, { target: { value: "theme" } });
    expect(screen.getByRole("button", { name: "styles/theme.css · Styles" })).toHaveAttribute(
      "data-emphasis",
      "match",
    );

    fireEvent.keyDown(input, { key: "Escape" });
    expect(input).toHaveValue("");
    expect(screen.getByRole("button", { name: "styles/theme.css · Styles" })).toHaveAttribute(
      "data-emphasis",
      "default",
    );
  });

  it("zoom controls operate without crashing the camera", () => {
    render(<ContextPage />);
    fireEvent.click(screen.getByRole("button", { name: "Zoom in" }));
    fireEvent.click(screen.getByRole("button", { name: "Zoom out" }));
    fireEvent.click(screen.getByRole("button", { name: "Fit to view" }));
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("drag pans the camera with the pointer and never rubber-bands back", () => {
    vi.useFakeTimers();
    const { container } = render(<ContextPage />);
    const canvas = container.querySelector<HTMLElement>('[tabindex="0"]');
    const layer = container.querySelector<HTMLElement>('[style*="will-change"]');
    expect(canvas).not.toBeNull();
    expect(layer).not.toBeNull();
    if (!canvas || !layer) return;

    fireEvent.pointerDown(canvas, { pointerId: 1, button: 0, clientX: 300, clientY: 200 });
    fireEvent.pointerMove(canvas, { pointerId: 1, clientX: 360, clientY: 240 });
    act(() => vi.advanceTimersByTime(20));

    // One flushed frame — the graph sits exactly under the pointer.
    expect(layer.style.transform).toBe("translate3d(60.00px, 40.00px, 0) scale(1)");

    // Even after the tween would have finished, the drag position holds.
    act(() => vi.advanceTimersByTime(700));
    expect(layer.style.transform).toBe("translate3d(60.00px, 40.00px, 0) scale(1)");
  });

  it("release after a fast drag coasts with momentum, then settles", () => {
    vi.useFakeTimers();
    const { container } = render(<ContextPage />);
    const canvas = container.querySelector<HTMLElement>('[tabindex="0"]');
    const layer = container.querySelector<HTMLElement>('[style*="will-change"]');
    if (!canvas || !layer) return;

    fireEvent.pointerDown(canvas, { pointerId: 1, button: 0, clientX: 300, clientY: 200 });
    fireEvent.pointerMove(canvas, { pointerId: 1, clientX: 360, clientY: 240 });
    fireEvent.pointerMove(canvas, { pointerId: 1, clientX: 400, clientY: 280 });
    fireEvent.pointerUp(canvas, { pointerId: 1, clientX: 400, clientY: 280 });
    act(() => vi.advanceTimersByTime(20));

    // Momentum carried the camera beyond the release point.
    expect(layer.style.transform).toBe("translate3d(140.00px, 120.00px, 0) scale(1)");

    // Momentum decays to rest; afterwards the camera is frozen.
    act(() => vi.advanceTimersByTime(1500));
    const settled = layer.style.transform;
    act(() => vi.advanceTimersByTime(1500));
    expect(layer.style.transform).toBe(settled);
  });

  it("camera loop survives the React StrictMode dev double-mount", () => {
    vi.useFakeTimers({
      toFake: ["setTimeout", "clearTimeout", "setInterval", "clearInterval", "requestAnimationFrame", "cancelAnimationFrame", "performance", "Date"],
    });
    const orig = HTMLElement.prototype.getBoundingClientRect;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.classList.contains("cursor-grab")) {
        return { width: 800, height: 600, left: 0, top: 0 } as DOMRect;
      }
      return orig.call(this);
    });
    const { container } = render(
      <StrictMode>
        <ContextPage />
      </StrictMode>,
    );
    const layer = container.querySelector<HTMLElement>('[style*="will-change"]');
    expect(layer).not.toBeNull();
    if (!layer) return;

    // StrictMode mounts → cleans up → remounts; the fit tween must still
    // run and the camera must settle at a scaled transform.
    act(() => vi.advanceTimersByTime(3000));
    expect(layer.style.transform).toMatch(/scale\(/);
    const settled = layer.style.transform;
    act(() => vi.advanceTimersByTime(1000));
    expect(layer.style.transform).toBe(settled);
    expect(settled).not.toBe("translate3d(0.00px, 0.00px, 0) scale(1)");
  });
});