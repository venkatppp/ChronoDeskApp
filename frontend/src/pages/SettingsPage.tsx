import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useTheme } from "@/hooks/useTheme";
import type { ThemePreference } from "@/contexts/ThemeContext";
import { Folder, Plus, Trash2, Moon, Sun, Monitor, Info, Shield, GitBranch, HardDrive, Palette } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { PageContainer } from "@/components/ui/PageContainer";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionLabel } from "@/components/ui/SectionLabel";
import { cn } from "@/utils/cn";

const THEME_OPTIONS: { id: ThemePreference; label: string; icon: typeof Sun; description: string }[] = [
  { id: "light", label: "Light", icon: Sun, description: "Bright, paper-like surfaces" },
  { id: "dark", label: "Dark", icon: Moon, description: "True-black canvas with glass" },
  { id: "system", label: "System", icon: Monitor, description: "Follow the macOS appearance" },
];

const MATERIAL_SAMPLES: { material: string; label: string; caption: string }[] = [
  { material: "glass-chrome", label: "Chrome", caption: "Sidebar & toolbars" },
  { material: "glass-panel", label: "Panel", caption: "Cards & lists" },
  { material: "glass-control", label: "Control", caption: "Buttons & inputs" },
];

export function SettingsPage() {
  const { preference: theme, resolvedTheme, setPreference: setTheme } = useTheme();
  const [watchPaths, setWatchPaths] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [appVersion, setAppVersion] = useState("0.1.0");

  const fetchWatchPaths = async () => {
    setIsLoading(true);
    try {
      const paths = await invoke<string[]>("list_watch_paths");
      setWatchPaths(paths);
    } catch (err) {
      console.error("Failed to list watch paths:", err);
      setError("Failed to load watched folders.");
    } finally {
      setIsLoading(false);
    }
  };

  const fetchAppVersion = async () => {
    try {
      const version = await invoke<string>("get_app_version");
      setAppVersion(version);
    } catch (err) {
      console.error("Failed to fetch app version:", err);
    }
  };

  useEffect(() => {
    fetchWatchPaths();
    fetchAppVersion();
  }, []);

  const handleAddPath = async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: "Select Folder" });
      if (!selected) return;
      await invoke("add_watch_path", { path: selected });
      fetchWatchPaths();
    } catch (err) {
      console.error("Failed to add watch path:", err);
    }
  };

  const handleRemovePath = async (path: string) => {
    try {
      await invoke("remove_watch_path", { path });
      fetchWatchPaths();
    } catch (err) {
      console.error("Failed to remove watch path:", err);
    }
  };

  const selectedOption = THEME_OPTIONS.find((option) => option.id === theme) ?? THEME_OPTIONS[1];

  return (
    <PageContainer>
      <PageHeader
        eyebrow="Preferences"
        title="Settings"
        description="Application appearance and the folders ChronoDesk watches for activity."
      />

      {error && (
        <div className="flex items-center gap-2.5 rounded-[var(--radius-card)] border border-(--color-danger)/30 bg-(--color-danger)/10 px-4 py-3 text-sm text-(--color-danger)">
          {error}
        </div>
      )}

      <div className="grid w-full grid-cols-1 items-start gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,22rem)]">
        {/* Main column — Appearance + Watched Folders; from xl up the two
            cards sit side by side so the pane uses the desktop width. */}
        <div className="flex min-w-0 flex-col gap-6 xl:grid xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] xl:gap-6">
          {/* Appearance */}
          <Card className="overflow-hidden">
            <div className="flex items-center gap-3 border-b border-(--color-border-subtle) px-5 py-4">
              <span className="flex h-9 w-9 items-center justify-center rounded-[var(--radius-control)] bg-(--color-surface-raised) text-(--color-muted-foreground)">
                <Monitor className="h-4.5 w-4.5" strokeWidth={1.75} />
              </span>
              <div>
                <h2 className="font-(family-name:--font-display) text-[15px] font-semibold tracking-tight text-(--color-foreground)">
                  Appearance
                </h2>
                <p className="text-[13px] text-(--color-muted-foreground)">Choose how ChronoDesk looks on your screen.</p>
              </div>
            </div>

            <div className="p-5">
              <SectionLabel className="mb-3">Theme</SectionLabel>
              <div role="radiogroup" aria-label="Theme" className="flex flex-col gap-2">
                {THEME_OPTIONS.map(({ id, label, icon: Icon, description }) => {
                  const selected = theme === id;
                  return (
                    <button
                      key={id}
                      type="button"
                      role="radio"
                      aria-checked={selected}
                      onClick={() => setTheme(id)}
                      className={cn(
                        "group flex w-full items-center gap-4 rounded-[var(--radius-control)] px-4 py-3 text-left transition-all duration-200 ease-[var(--ease-premium)]",
                        selected
                          ? "material-selected"
                          : "bg-transparent hover:bg-(--color-surface-hover)",
                      )}
                    >
                      <span
                        className={cn(
                          "flex h-9 w-9 shrink-0 items-center justify-center rounded-[var(--radius-control)] transition-colors",
                          selected
                            ? "glass-accent text-(--color-accent-foreground)"
                            : "bg-(--color-surface-raised)/70 text-(--color-muted-foreground) group-hover:text-(--color-foreground)",
                        )}
                      >
                        <Icon className="h-4.5 w-4.5" strokeWidth={1.75} />
                      </span>
                      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                        <span className="text-sm font-medium text-(--color-foreground)">{label}</span>
                        <span className="text-xs leading-snug text-(--color-muted-foreground)">{description}</span>
                      </span>
                      <span
                        className={cn(
                          "flex h-4 w-4 shrink-0 items-center justify-center rounded-full border transition-colors",
                          selected
                            ? "border-(--color-accent) bg-(--color-accent)/25"
                            : "border-(--color-border) group-hover:border-(--color-faint-foreground)",
                        )}
                        aria-hidden="true"
                      >
                        {selected && <span className="h-1.5 w-1.5 rounded-full bg-(--color-accent)" />}
                      </span>
                    </button>
                  );
                })}
              </div>
            </div>
          </Card>

          {/* Watched Folders */}
          <Card className="overflow-hidden">
            <div className="flex items-center justify-between gap-3 border-b border-(--color-border-subtle) px-5 py-4">
              <div className="flex min-w-0 items-center gap-3">
                <span className="flex h-9 w-9 items-center justify-center rounded-[var(--radius-control)] bg-(--color-surface-raised) text-(--color-muted-foreground)">
                  <HardDrive className="h-4.5 w-4.5" strokeWidth={1.75} />
                </span>
                <div className="min-w-0">
                  <h2 className="font-(family-name:--font-display) text-[15px] font-semibold tracking-tight text-(--color-foreground)">
                    Watched Folders
                  </h2>
                  <p className="truncate text-[13px] text-(--color-muted-foreground)">
                    Directories ChronoDesk monitors for changes across every workspace.
                  </p>
                </div>
              </div>
              <div className="flex shrink-0 items-center gap-2.5">
                {!isLoading && watchPaths.length > 0 && (
                  <span className="rounded-full border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1 text-[11px] tabular-nums text-(--color-muted-foreground)">
                    {watchPaths.length} watched
                  </span>
                )}
                <Button onClick={handleAddPath} size="sm">
                  <Plus className="h-3.5 w-3.5" strokeWidth={1.75} />
                  Add Folder
                </Button>
              </div>
            </div>

            <div className="p-5">
              {isLoading ? (
                <div className="space-y-2.5">
                  {[...Array(2)].map((_, i) => (
                    <div key={i} className="h-12 animate-pulse rounded-[var(--radius-control)] bg-(--color-surface)" />
                  ))}
                </div>
              ) : watchPaths.length === 0 ? (
                <div className="flex flex-col items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-6 py-10 text-center">
                  <Folder className="h-6 w-6 text-(--color-faint-foreground)" strokeWidth={1.5} />
                  <p className="text-sm font-medium text-(--color-foreground)">No folders watched yet</p>
                  <p className="max-w-sm text-xs leading-relaxed text-(--color-muted-foreground)">
                    Add a folder to start capturing file activity for your workspaces.
                  </p>
                </div>
              ) : (
                <div className="flex flex-col gap-2">
                  {watchPaths.map((path) => (
                    <div
                      key={path}
                      className="group flex items-center gap-3 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-3.5 py-2.5 transition-colors hover:border-(--color-border)"
                    >
                      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[var(--radius-control)] bg-(--color-accent-muted) text-(--color-accent)">
                        <Folder className="h-4 w-4" strokeWidth={1.75} />
                      </span>
                      <span className="min-w-0 flex-1 truncate font-(family-name:--font-mono) text-xs text-(--color-foreground)">
                        {path}
                      </span>
                      <button
                        onClick={() => handleRemovePath(path)}
                        className="rounded-lg p-1.5 text-(--color-faint-foreground) opacity-0 transition-all hover:bg-(--color-danger)/10 hover:text-(--color-danger) group-hover:opacity-100"
                        aria-label={`Remove ${path}`}
                        title="Stop watching"
                      >
                        <Trash2 className="h-4 w-4" strokeWidth={1.75} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </Card>
        </div>

        {/* Rail column — live appearance state */}
        <div className="flex min-w-0 flex-col gap-6">
          <Card className="overflow-hidden">
            <div className="flex items-center gap-3 border-b border-(--color-border-subtle) px-5 py-4">
              <span className="flex h-9 w-9 items-center justify-center rounded-[var(--radius-control)] bg-(--color-surface-raised) text-(--color-muted-foreground)">
                <Palette className="h-4.5 w-4.5" strokeWidth={1.75} />
              </span>
              <div>
                <h2 className="font-(family-name:--font-display) text-[15px] font-semibold tracking-tight text-(--color-foreground)">
                  Current appearance
                </h2>
                <p className="text-[13px] text-(--color-muted-foreground)">How the window renders right now.</p>
              </div>
            </div>

            <div className="p-5">
              <div className="flex items-center justify-between gap-3">
                <span className="flex items-center gap-2.5">
                  <span className="h-2 w-2 rounded-full bg-(--color-accent)" />
                  <span className="text-sm font-semibold capitalize text-(--color-foreground)">{resolvedTheme}</span>
                </span>
                {theme === "system" && (
                  <span className="rounded-full border border-(--color-border-subtle) bg-(--color-surface) px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-(--color-muted-foreground)">
                    Following system
                  </span>
                )}
              </div>
              <p className="mt-2 text-xs leading-relaxed text-(--color-muted-foreground)">
                {selectedOption.description}
              </p>

              <div className="mt-5 rounded-[var(--radius-control)] bg-env p-3">
                <p className="mb-2.5 text-[10px] font-semibold uppercase tracking-widest text-(--color-faint-foreground)">
                  Material preview
                </p>
                <div className="grid grid-cols-3 gap-2.5">
                  {MATERIAL_SAMPLES.map((sample) => (
                    <div key={sample.material} className="flex min-w-0 flex-col items-center gap-1.5">
                      <div className={cn("h-11 w-full rounded-[0.625rem]", sample.material)} />
                      <span className="text-[10px] font-medium text-(--color-muted-foreground)">{sample.label}</span>
                      <span className="truncate text-[9px] text-(--color-faint-foreground)">{sample.caption}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </Card>
        </div>

        {/* About — full-width strip */}
        <Card className="overflow-hidden lg:col-span-2">
          <div className="flex flex-wrap items-center justify-between gap-4 px-5 py-5">
            <div className="flex min-w-0 items-center gap-4">
              <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-raised) text-(--color-muted-foreground)">
                <Shield className="h-5 w-5" strokeWidth={1.75} />
              </span>
              <div className="min-w-0">
                <h2 className="font-(family-name:--font-display) text-base font-bold tracking-tight text-(--color-foreground)">
                  ChronoDesk
                </h2>
                <p className="text-xs text-(--color-muted-foreground)">
                  Version {appVersion} <span className="mx-1 text-(--color-faint-foreground)">·</span> Workspace layer
                </p>
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <Button variant="ghost" size="icon" aria-label="About" title="About ChronoDesk">
                <Info className="h-4 w-4" strokeWidth={1.75} />
              </Button>
              <Button variant="ghost" size="icon" aria-label="Source" title="Source repository">
                <GitBranch className="h-4 w-4" strokeWidth={1.75} />
              </Button>
            </div>
          </div>
          <div className="flex items-center justify-between border-t border-(--color-border-subtle) px-5 py-3 text-[10px] font-semibold uppercase tracking-widest text-(--color-faint-foreground)">
            <span>© 2026 ChronoDesk Labs</span>
            <span>Privacy · Terms</span>
          </div>
        </Card>
      </div>
    </PageContainer>
  );
}
