// JournalPanel tests — the append-only reliability ledger renders entry
// types, entities, states and scopes.

import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { JournalPanel } from "./JournalPanel";
import type { RecoveryJournalEntry } from "@/types/recovery";

const entries: RecoveryJournalEntry[] = [
  {
    id: 7,
    entryType: "checkpoint",
    scope: "startup",
    entity: "app",
    state: "running",
    payload: { active_jobs: ["job-1"] },
    checksum: "abc123",
    createdAt: "2026-08-04T11:58:00Z",
  },
  {
    id: 8,
    entryType: "self_healing",
    scope: "watchdog",
    entity: "indexer",
    state: "stalled",
    payload: {},
    checksum: "def456",
    createdAt: "2026-08-04T12:00:00Z",
  },
];

describe("JournalPanel", () => {
  it("renders entries with type badges, entities and states", () => {
    render(<JournalPanel entries={entries} loading={false} error={null} />);
    expect(screen.getByText("Reliability Journal")).toBeInTheDocument();
    expect(screen.getByText("checkpoint")).toBeInTheDocument();
    expect(screen.getByText("self-healing")).toBeInTheDocument();
    expect(screen.getAllByText("app").length).toBeGreaterThan(0);
    expect(screen.getByText("stalled")).toBeInTheDocument();
    expect(screen.getByText("abc123")).toBeInTheDocument();
  });

  it("handles the empty, loading and error states", () => {
    render(<JournalPanel entries={[]} loading={false} error={null} />);
    expect(screen.getByText(/No journal entries/)).toBeInTheDocument();
    render(<JournalPanel entries={[]} loading error={null} />);
    expect(screen.getByText(/Loading journal/)).toBeInTheDocument();
    render(<JournalPanel entries={[]} loading={false} error="boom" />);
    expect(screen.getByText("boom")).toBeInTheDocument();
  });
});