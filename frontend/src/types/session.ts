// Session Intelligence Types
//
// TypeScript types matching the Rust session models for IPC communication.

export interface ScoreFactor {
  name: string;
  weight: number;
  value: number;
  reason: string;
}

export interface SessionEventSummary {
  occurredAt: string; // ISO 8601 datetime
  eventType: string;
  fileName?: string;
  description: string;
}

export interface SessionSummary {
  workspaceId: string;
  workspaceName: string;
  startedAt: string; // ISO 8601 datetime
  endedAt: string; // ISO 8601 datetime
  durationSeconds: number;
  fileCount: number;
  languages: string[];
  productivityScore: number;
  scoreFactors: ScoreFactor[];
  recentEvents: SessionEventSummary[];
}

export interface Session {
  workspaceId: string;
  startedAt: string;
  endedAt: string;
  durationSeconds: number;
  eventCount: number;
  fileCount: number;
  languages: string[];
  productivityScore?: {
    score: number;
    factors: ScoreFactor[];
  };
}
