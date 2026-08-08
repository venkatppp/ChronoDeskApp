import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface ModelMetadata {
  id: string;
  name: string;
  modelType: 'embedding' | 'reranker';
  version: string;
  dimensions: number;
  maxSequenceLength: number;
  fileSizeBytes: number;
  downloadUrl: string;
  tokenizerUrl?: string;
  description: string;
}

interface ModelInfo {
  metadata: ModelMetadata;
  status: 'not_downloaded' | 'downloading' | 'downloaded' | 'loading' | 'loaded' | 'unloading' | 'error';
  localPath?: string;
  downloadedAt?: string;
  loadedAt?: string;
  memoryUsageBytes?: number;
  errorMessage?: string;
}

interface DownloadProgress {
  modelId: string;
  bytesDownloaded: number;
  totalBytes: number;
  progressPercent: number;
  speedBytesPerSec: number;
}

interface InferenceStats {
  modelId: string;
  modelType: 'embedding' | 'reranker';
  totalInferences: number;
  cacheHits: number;
  cacheMisses: number;
  cacheHitRate: number;
  avgLatencyMs: number;
  p50LatencyMs: number;
  p95LatencyMs: number;
  p99LatencyMs: number;
  lastInferenceAt?: string;
}

interface AIDiagnostics {
  models: ModelInfo[];
  inferenceStats: Record<string, InferenceStats>;
  totalMemoryUsageBytes: number;
  cacheStats: {
    embeddingCacheSize: number;
    embeddingCacheCapacity: number;
    embeddingCacheHitRate: number;
    inferenceCacheSize: number;
    inferenceCacheCapacity: number;
    inferenceCacheHitRate: number;
  };
  systemInfo: {
    availableMemoryBytes: number;
    cpuCores: number;
    onnxRuntimeVersion: string;
  };
}

const AIModelsPage: React.FC = () => {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [diagnostics, setDiagnostics] = useState<AIDiagnostics | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, DownloadProgress>>({});
  const [activeEmbeddingModel, setActiveEmbeddingModel] = useState<string | null>(null);
  const [activeRerankerModel, setActiveRerankerModel] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadModels();
    loadDiagnostics();
    loadActiveModels();

    // Listen for download progress events
    const unlistenPromise = listen<DownloadProgress>('model:download_progress', (event) => {
      setDownloadProgress((prev) => ({
        ...prev,
        [event.payload.modelId]: event.payload,
      }));
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const loadModels = async () => {
    try {
      setLoading(true);
      const modelsList = await invoke<ModelInfo[]>('list_models');
      setModels(modelsList);
      setError(null);
    } catch (err) {
      setError(`Failed to load models: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const loadDiagnostics = async () => {
    try {
      const diag = await invoke<AIDiagnostics>('get_ai_diagnostics');
      setDiagnostics(diag);
    } catch (err) {
      console.error('Failed to load diagnostics:', err);
    }
  };

  const loadActiveModels = async () => {
    try {
      const embeddingModel = await invoke<string | null>('get_active_embedding_model');
      const rerankerModel = await invoke<string | null>('get_active_reranker_model');
      setActiveEmbeddingModel(embeddingModel);
      setActiveRerankerModel(rerankerModel);
    } catch (err) {
      console.error('Failed to load active models:', err);
    }
  };

  const handleDownload = async (modelId: string) => {
    try {
      await invoke('download_model', { modelId });
      await loadModels();
    } catch (err) {
      setError(`Failed to download model: ${err}`);
    }
  };

  const handleLoad = async (modelId: string) => {
    try {
      await invoke('load_model', { modelId });
      await loadModels();
      await loadActiveModels();
      await loadDiagnostics();
    } catch (err) {
      setError(`Failed to load model: ${err}`);
    }
  };

  const handleUnload = async (modelId: string) => {
    try {
      await invoke('unload_model', { modelId });
      await loadModels();
      await loadActiveModels();
      await loadDiagnostics();
    } catch (err) {
      setError(`Failed to unload model: ${err}`);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const getStatusBadgeClass = (status: string): string => {
    switch (status) {
      case 'loaded':
        return 'bg-green-100 text-green-800';
      case 'downloading':
      case 'loading':
        return 'bg-blue-100 text-blue-800';
      case 'downloaded':
        return 'bg-yellow-100 text-yellow-800';
      case 'error':
        return 'bg-red-100 text-red-800';
      default:
        return 'bg-(--color-surface-hover) text-(--color-foreground)';
    }
  };

  const getStatusText = (status: string): string => {
    return status.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase());
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-(--color-muted-foreground)">Loading AI models...</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-(--color-foreground)">AI Models</h1>
        <p className="text-(--color-muted-foreground) mt-1">Manage local AI models for embeddings and reranking</p>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
          {error}
        </div>
      )}

      {/* System Diagnostics */}
      {diagnostics && (
        <div className="bg-(--color-surface) rounded-lg shadow p-6 space-y-4">
          <h2 className="text-lg font-semibold text-(--color-foreground)">System Diagnostics</h2>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <div className="text-sm text-(--color-muted-foreground)">Total Memory Usage</div>
              <div className="text-xl font-semibold text-(--color-foreground)">
                {formatBytes(diagnostics.totalMemoryUsageBytes)}
              </div>
            </div>
            <div>
              <div className="text-sm text-(--color-muted-foreground)">Available Memory</div>
              <div className="text-xl font-semibold text-(--color-foreground)">
                {formatBytes(diagnostics.systemInfo.availableMemoryBytes)}
              </div>
            </div>
            <div>
              <div className="text-sm text-(--color-muted-foreground)">CPU Cores</div>
              <div className="text-xl font-semibold text-(--color-foreground)">
                {diagnostics.systemInfo.cpuCores}
              </div>
            </div>
          </div>
          <div className="pt-4 border-t">
            <div className="text-sm text-(--color-muted-foreground)">ONNX Runtime Version</div>
            <div className="text-sm font-medium text-(--color-foreground)">
              {diagnostics.systemInfo.onnxRuntimeVersion}
            </div>
          </div>
        </div>
      )}

      {/* Active Models */}
      <div className="bg-(--color-surface) rounded-lg shadow p-6 space-y-3">
        <h2 className="text-lg font-semibold text-(--color-foreground)">Active Models</h2>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <div className="text-sm text-(--color-muted-foreground)">Active Embedding Model</div>
            <div className="text-sm font-medium text-(--color-foreground)">
              {activeEmbeddingModel || 'None (using local fallback)'}
            </div>
          </div>
          <div>
            <div className="text-sm text-(--color-muted-foreground)">Active Reranker Model</div>
            <div className="text-sm font-medium text-(--color-foreground)">
              {activeRerankerModel || 'None'}
            </div>
          </div>
        </div>
      </div>

      {/* Model List */}
      <div className="space-y-4">
        <h2 className="text-lg font-semibold text-(--color-foreground)">Available Models</h2>
        {models.map((model) => (
          <div key={model.metadata.id} className="bg-(--color-surface) rounded-lg shadow p-6">
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <div className="flex items-center gap-3">
                  <h3 className="text-lg font-semibold text-(--color-foreground)">
                    {model.metadata.name}
                  </h3>
                  <span
                    className={`px-2 py-1 text-xs font-medium rounded ${getStatusBadgeClass(
                      model.status
                    )}`}
                  >
                    {getStatusText(model.status)}
                  </span>
                  <span className="px-2 py-1 text-xs font-medium bg-blue-100 text-blue-800 rounded">
                    {model.metadata.modelType}
                  </span>
                </div>
                <p className="text-sm text-(--color-muted-foreground) mt-1">{model.metadata.description}</p>
                <div className="grid grid-cols-3 gap-4 mt-3 text-sm">
                  <div>
                    <span className="text-(--color-muted-foreground)">Version:</span>{' '}
                    <span className="text-(--color-foreground)">{model.metadata.version}</span>
                  </div>
                  <div>
                    <span className="text-(--color-muted-foreground)">Dimensions:</span>{' '}
                    <span className="text-(--color-foreground)">{model.metadata.dimensions}</span>
                  </div>
                  <div>
                    <span className="text-(--color-muted-foreground)">Size:</span>{' '}
                    <span className="text-(--color-foreground)">
                      {formatBytes(model.metadata.fileSizeBytes)}
                    </span>
                  </div>
                </div>
                {model.memoryUsageBytes && (
                  <div className="mt-2 text-sm">
                    <span className="text-(--color-muted-foreground)">Memory Usage:</span>{' '}
                    <span className="text-(--color-foreground)">{formatBytes(model.memoryUsageBytes)}</span>
                  </div>
                )}
                {model.errorMessage && (
                  <div className="mt-2 text-sm text-(--color-danger)">{model.errorMessage}</div>
                )}
                {downloadProgress[model.metadata.id] && (
                  <div className="mt-3">
                    <div className="flex justify-between text-sm mb-1">
                      <span className="text-(--color-muted-foreground)">Downloading...</span>
                      <span className="text-(--color-foreground)">
                        {downloadProgress[model.metadata.id].progressPercent.toFixed(1)}%
                      </span>
                    </div>
                    <div className="w-full bg-(--color-surface-hover) rounded-full h-2">
                      <div
                        className="bg-(--color-accent) h-2 rounded-full transition-all duration-300"
                        style={{
                          width: `${downloadProgress[model.metadata.id].progressPercent}%`,
                        }}
                      />
                    </div>
                  </div>
                )}
              </div>
              <div className="flex gap-2 ml-4">
                {model.status === 'not_downloaded' && (
                  <button
                    onClick={() => handleDownload(model.metadata.id)}
                    className="px-4 py-2 bg-(--color-accent) text-white rounded hover:bg-(--color-accent) transition"
                  >
                    Download
                  </button>
                )}
                {model.status === 'downloaded' && (
                  <button
                    onClick={() => handleLoad(model.metadata.id)}
                    className="px-4 py-2 bg-(--color-success) text-white rounded hover:bg-green-700 transition"
                  >
                    Load
                  </button>
                )}
                {model.status === 'loaded' && (
                  <button
                    onClick={() => handleUnload(model.metadata.id)}
                    className="px-4 py-2 rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface-hover) text-(--color-muted-foreground) transition hover:text-(--color-foreground)"
                  >
                    Unload
                  </button>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Inference Statistics */}
      {diagnostics && Object.keys(diagnostics.inferenceStats).length > 0 && (
        <div className="bg-(--color-surface) rounded-lg shadow p-6 space-y-4">
          <h2 className="text-lg font-semibold text-(--color-foreground)">Inference Statistics</h2>
          {Object.values(diagnostics.inferenceStats).map((stats) => (
            <div key={stats.modelId} className="border-t pt-4 first:border-t-0 first:pt-0">
              <h3 className="text-sm font-semibold text-(--color-foreground) mb-3">{stats.modelId}</h3>
              <div className="grid grid-cols-4 gap-4 text-sm">
                <div>
                  <div className="text-(--color-muted-foreground)">Total Inferences</div>
                  <div className="text-(--color-foreground) font-medium">{stats.totalInferences}</div>
                </div>
                <div>
                  <div className="text-(--color-muted-foreground)">Cache Hit Rate</div>
                  <div className="text-(--color-foreground) font-medium">
                    {(stats.cacheHitRate * 100).toFixed(1)}%
                  </div>
                </div>
                <div>
                  <div className="text-(--color-muted-foreground)">Avg Latency</div>
                  <div className="text-(--color-foreground) font-medium">{stats.avgLatencyMs.toFixed(2)} ms</div>
                </div>
                <div>
                  <div className="text-(--color-muted-foreground)">P95 Latency</div>
                  <div className="text-(--color-foreground) font-medium">{stats.p95LatencyMs.toFixed(2)} ms</div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default AIModelsPage;
