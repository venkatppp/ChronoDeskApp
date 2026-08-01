import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Brain,
  TrendingUp,
  Target,
  Activity,
  CheckCircle,
  XCircle,
  Clock,
  BarChart3,
} from 'lucide-react';

interface LearningStats {
  total_feedback_count: number;
  accepted_count: number;
  rejected_count: number;
  acceptance_rate: number;
  total_preferences: number;
  total_patterns: number;
  avg_confidence_adjustment: number;
  last_learning_update: string;
}

interface UserPreference {
  id: string;
  preference_type: string;
  key: string;
  value: string;
  confidence: number;
  evidence_count: number;
  last_updated: string;
}

interface BehavioralPattern {
  id: string;
  pattern_type: string;
  description: string;
  conditions: Record<string, unknown>;
  frequency: number;
  confidence: number;
  occurrences: number;
  first_seen: string;
  last_seen: string;
}

interface ConfidenceTrend {
  date: string;
  avg_confidence: number;
  adjustment_count: number;
}

interface CategoryAccuracy {
  category: string;
  accuracy: number;
  total: number;
  accepted: number;
}

interface RecommendationAccuracy {
  category_accuracy: CategoryAccuracy[];
  overall_accuracy: number;
  total_recommendations: number;
}

interface LearningInsights {
  stats: LearningStats;
  top_preferences: UserPreference[];
  recent_patterns: BehavioralPattern[];
  confidence_trends: ConfidenceTrend[];
  recommendation_accuracy: RecommendationAccuracy;
}

export default function LearningPage() {
  const [insights, setInsights] = useState<LearningInsights | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadInsights();
  }, []);

  async function loadInsights() {
    try {
      setLoading(true);
      setError(null);
      const data = await invoke<LearningInsights>('get_learning_insights');
      setInsights(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p className="text-gray-600 dark:text-gray-400">Loading learning insights...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <XCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
          <p className="text-red-600 dark:text-red-400 mb-4">{error}</p>
          <button
            onClick={loadInsights}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (!insights) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-gray-600 dark:text-gray-400">No learning data available</p>
      </div>
    );
  }

  const { stats, top_preferences, recent_patterns, confidence_trends, recommendation_accuracy } = insights;

  return (
    <div className="p-6 space-y-6 overflow-y-auto h-full">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
            <Brain className="w-8 h-8 text-purple-600" />
            Learning Insights
          </h1>
          <p className="text-gray-600 dark:text-gray-400 mt-1">
            Adaptive learning from your behavior and feedback
          </p>
        </div>
        <button
          onClick={loadInsights}
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
        >
          Refresh
        </button>
      </div>

      {/* Stats Overview */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-600 dark:text-gray-400">Total Feedback</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {stats.total_feedback_count}
              </p>
            </div>
            <Activity className="w-8 h-8 text-blue-600" />
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-600 dark:text-gray-400">Acceptance Rate</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {(stats.acceptance_rate * 100).toFixed(1)}%
              </p>
            </div>
            <CheckCircle className="w-8 h-8 text-green-600" />
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-600 dark:text-gray-400">Preferences</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {stats.total_preferences}
              </p>
            </div>
            <Target className="w-8 h-8 text-purple-600" />
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-600 dark:text-gray-400">Patterns</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {stats.total_patterns}
              </p>
            </div>
            <BarChart3 className="w-8 h-8 text-orange-600" />
          </div>
        </div>
      </div>

      {/* Recommendation Accuracy */}
      <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
        <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          <TrendingUp className="w-5 h-5 text-green-600" />
          Recommendation Accuracy
        </h2>
        <div className="mb-4">
          <div className="flex justify-between items-center mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Overall Accuracy</span>
            <span className="text-lg font-semibold text-gray-900 dark:text-white">
              {(recommendation_accuracy.overall_accuracy * 100).toFixed(1)}%
            </span>
          </div>
          <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
            <div
              className="bg-green-600 h-2 rounded-full"
              style={{ width: `${recommendation_accuracy.overall_accuracy * 100}%` }}
            ></div>
          </div>
        </div>
        <div className="space-y-3">
          {recommendation_accuracy.category_accuracy.map((cat) => (
            <div key={cat.category}>
              <div className="flex justify-between items-center mb-1">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  {cat.category}
                </span>
                <span className="text-sm text-gray-600 dark:text-gray-400">
                  {cat.accepted}/{cat.total} ({(cat.accuracy * 100).toFixed(0)}%)
                </span>
              </div>
              <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                <div
                  className="bg-blue-600 h-1.5 rounded-full"
                  style={{ width: `${cat.accuracy * 100}%` }}
                ></div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Top Preferences */}
      <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
        <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          <Target className="w-5 h-5 text-purple-600" />
          Top Preferences
        </h2>
        {top_preferences.length === 0 ? (
          <p className="text-gray-600 dark:text-gray-400">No preferences learned yet</p>
        ) : (
          <div className="space-y-3">
            {top_preferences.map((pref) => (
              <div
                key={pref.id}
                className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-md"
              >
                <div className="flex-1">
                  <p className="font-medium text-gray-900 dark:text-white">{pref.key}</p>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {pref.preference_type} • {pref.evidence_count} occurrences
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-sm font-semibold text-gray-900 dark:text-white">
                    {(pref.confidence * 100).toFixed(0)}%
                  </p>
                  <p className="text-xs text-gray-600 dark:text-gray-400">confidence</p>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Recent Patterns */}
      <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
        <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          <Activity className="w-5 h-5 text-orange-600" />
          Behavioral Patterns
        </h2>
        {recent_patterns.length === 0 ? (
          <p className="text-gray-600 dark:text-gray-400">No patterns detected yet</p>
        ) : (
          <div className="space-y-3">
            {recent_patterns.map((pattern) => (
              <div
                key={pattern.id}
                className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-md"
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex-1">
                    <p className="font-medium text-gray-900 dark:text-white">{pattern.description}</p>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {pattern.pattern_type} • {pattern.occurrences} occurrences
                    </p>
                  </div>
                  <span className="text-sm font-semibold text-gray-900 dark:text-white">
                    {(pattern.confidence * 100).toFixed(0)}%
                  </span>
                </div>
                <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                  <Clock className="w-3 h-3" />
                  <span>Frequency: {pattern.frequency.toFixed(2)}</span>
                  <span>•</span>
                  <span>Last seen: {new Date(pattern.last_seen).toLocaleDateString()}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Confidence Trends */}
      <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
        <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          <TrendingUp className="w-5 h-5 text-blue-600" />
          Confidence Trends (30 days)
        </h2>
        {confidence_trends.length === 0 ? (
          <p className="text-gray-600 dark:text-gray-400">No trend data available yet</p>
        ) : (
          <div className="space-y-2">
            {confidence_trends.map((trend) => (
              <div key={trend.date} className="flex items-center gap-3">
                <span className="text-sm text-gray-600 dark:text-gray-400 w-24">
                  {new Date(trend.date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
                </span>
                <div className="flex-1 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                  <div
                    className="bg-blue-600 h-2 rounded-full"
                    style={{ width: `${trend.avg_confidence * 100}%` }}
                  ></div>
                </div>
                <span className="text-sm font-medium text-gray-900 dark:text-white w-16 text-right">
                  {(trend.avg_confidence * 100).toFixed(0)}%
                </span>
                <span className="text-xs text-gray-600 dark:text-gray-400 w-16 text-right">
                  {trend.adjustment_count} adj.
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Feedback Summary */}
      <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
        <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4">Feedback Summary</h2>
        <div className="grid grid-cols-2 gap-4">
          <div className="flex items-center gap-3">
            <CheckCircle className="w-8 h-8 text-green-600" />
            <div>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {stats.accepted_count}
              </p>
              <p className="text-sm text-gray-600 dark:text-gray-400">Accepted</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <XCircle className="w-8 h-8 text-red-600" />
            <div>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {stats.rejected_count}
              </p>
              <p className="text-sm text-gray-600 dark:text-gray-400">Rejected</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
