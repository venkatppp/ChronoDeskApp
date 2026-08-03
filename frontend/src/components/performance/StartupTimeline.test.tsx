// StartupTimeline tests — the launch stages render as proportional bars,
// the slowest stage is surfaced, and clicking a bar pins its detail.

import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { StartupTimeline } from "./StartupTimeline";
import type { StartupProfile } from "@/types/performance";

const profile: StartupProfile = {
  runId: "run-1",
  totalMs: 1600,
  stages: [
    { name: "database", label: "Database initialization", durationMs: 200, startedAt: "2026-08-03T11:00:00Z" },
    { name: "graph_sync", label: "Initial knowledge graph sync", durationMs: 900, startedAt: "2026-08-03T11:00:00Z" },
    { name: "copilot", label: "Copilot engine", durationMs: 300, startedAt: "2026-08-03T11:00:01Z" },
  ],
  recordedAt: "2026-08-03T11:00:02Z",
};

describe("StartupTimeline", () => {
  it("renders every stage with the total", () => {
    render(<StartupTimeline profiles={[profile]} loading={false} error={null} />);
    expect(screen.getByText("Startup Timeline")).toBeInTheDocument();
    expect(screen.getByText(/1,600 ms across 3 stages/)).toBeInTheDocument();
    expect(screen.getByText("Database initialization")).toBeInTheDocument();
    expect(screen.getByText("Copilot engine")).toBeInTheDocument();
  });

  it("highlights the slowest stage", () => {
    render(<StartupTimeline profiles={[profile]} loading={false} error={null} />);
    expect(screen.getByText(/Slowest stage:/)).toBeInTheDocument();
    expect(screen.getAllByText("Initial knowledge graph sync").length).toBeGreaterThan(0);
  });

  it("pins the selected stage detail on click", () => {
    render(<StartupTimeline profiles={[profile]} loading={false} error={null} />);
    fireEvent.click(screen.getByText("Copilot engine"));
    expect(screen.getByText(/took 300 ms/)).toBeInTheDocument();
  });

  it("shows empty and loading states without crashing", () => {
    render(<StartupTimeline profiles={[]} loading error={null} />);
    expect(screen.getByText(/Loading startup timeline/)).toBeInTheDocument();
  });
});