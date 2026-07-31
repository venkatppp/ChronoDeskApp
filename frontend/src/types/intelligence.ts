/**
 * Intelligence Layer Types
 * 
 * TypeScript types for workspace health monitoring and recommendations.
 * Mirrors the Rust backend's intelligence module types.
 */

// ============================================================================
// Workspace Health Types
// ============================================================================

export interface HealthMetric {
  id: string;
  name: string;
  value: number;
  idealValue: number | null;
  unit: string;
}

export interface HealthFactor {
  id: string;
  name: string;
  description: string;
  score: number;
  weight: number;
  metrics: HealthMetric[];
}

export interface WorkspaceHealth {
  workspaceId: number;
  overallScore: number;
  factors: HealthFactor[];
  calculatedAt: string; // ISO-8601
  trend: number | null;
}

// ============================================================================
// Recommendation Types
// ============================================================================

export type RecommendationCategory =
  | "organization"
  | "productivity"
  | "context"
  | "files"
  | "search"
  | "health";

export type RecommendationPriority = "low" | "medium" | "high" | "critical";

export type RecommendationAction =
  | { type: "navigate"; path: string }
  | { type: "open_view"; view: string }
  | { type: "execute_command"; command: string; args: string[] }
  | { type: "info" }
  | { type: "custom"; data: unknown };

export interface Recommendation {
  id: string;
  workspaceId: number;
  category: RecommendationCategory;
  priority: RecommendationPriority;
  title: string;
  description: string;
  action: RecommendationAction;
  confidence: number;
  impact: number;
  effort: number;
  generatedAt: string; // ISO-8601
  expiresAt: string | null; // ISO-8601
  metadata: unknown;
}
