import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "@/hooks/useTheme";
import type { ThemePreference } from "@/contexts/ThemeContext";
import { Folder, Plus, Trash2, Moon, Sun, Monitor, Info, Shield, GitBranch } from "lucide-react";

export function SettingsPage() {
  const { preference: theme, setPreference: setTheme } = useTheme();
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
      // Typically we'd use tauri-plugin-dialog to pick a folder
      // For now we'll assume a path is provided or the command handles it
      await invoke("add_watch_path", { path: "/Users/srivenkat/AccioWork/chronodesk/" });
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

  return (
    <div className="max-w-4xl mx-auto px-6 py-10">
      <div className="mb-10">
        <h1 className="text-4xl font-bold text-foreground mb-2">Settings</h1>
        <p className="text-muted-foreground text-lg">Configure your workspace preferences and watched directories.</p>
      </div>

      {error && (
        <div className="mb-6 p-4 bg-destructive/10 border border-destructive/20 rounded-xl text-destructive text-sm">
          {error}
        </div>
      )}

      <div className="space-y-8">
        {/* Watched Folders */}
        <section className="bg-background-secondary border border-border rounded-3xl overflow-hidden">
          <div className="p-8 border-b border-border bg-background-tertiary/30">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2.5 bg-primary/10 text-primary rounded-xl">
                  <Folder className="h-6 w-6" />
                </div>
                <div>
                  <h2 className="text-xl font-bold text-foreground">Watched Folders</h2>
                  <p className="text-sm text-muted-foreground">Directories ChronoDesk monitors for changes.</p>
                </div>
              </div>
              <button 
                onClick={handleAddPath}
                className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg font-bold text-sm hover:scale-[1.02] active:scale-[0.98] transition-all"
              >
                <Plus className="h-4 w-4" />
                Add Folder
              </button>
            </div>
          </div>
          
          <div className="p-8">
            {isLoading ? (
              <div className="space-y-3">
                {[...Array(2)].map((_, i) => (
                  <div key={i} className="h-12 bg-background-tertiary rounded-xl animate-pulse" />
                ))}
              </div>
            ) : watchPaths.length === 0 ? (
              <div className="py-10 text-center opacity-40">
                <p className="text-sm italic">No folders are being watched yet.</p>
              </div>
            ) : (
              <div className="space-y-3">
                {watchPaths.map((path) => (
                  <div key={path} className="group flex items-center justify-between p-4 bg-background-tertiary/50 border border-border rounded-2xl hover:border-primary/30 transition-all">
                    <div className="flex items-center gap-3 min-w-0">
                      <Folder className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm font-mono text-foreground truncate">{path}</span>
                    </div>
                    <button 
                      onClick={() => handleRemovePath(path)}
                      className="p-2 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded-lg transition-all opacity-0 group-hover:opacity-100"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </section>

        {/* Appearance */}
        <section className="bg-background-secondary border border-border rounded-3xl overflow-hidden">
          <div className="p-8 border-b border-border bg-background-tertiary/30">
            <div className="flex items-center gap-3">
              <div className="p-2.5 bg-blue-500/10 text-blue-500 rounded-xl">
                <Monitor className="h-6 w-6" />
              </div>
              <div>
                <h2 className="text-xl font-bold text-foreground">Appearance</h2>
                <p className="text-sm text-muted-foreground">Customize how ChronoDesk looks on your screen.</p>
              </div>
            </div>
          </div>
          
          <div className="p-8">
            <div className="grid grid-cols-3 gap-4">
              {[
                { id: "light", icon: Sun, label: "Light" },
                { id: "dark", icon: Moon, label: "Dark" },
                { id: "system", icon: Monitor, label: "System" },
              ].map((item) => (
                <button
                  key={item.id}
                  onClick={() => setTheme(item.id as ThemePreference)}
                  className={`flex flex-col items-center gap-3 p-6 rounded-2xl border-2 transition-all ${
                    theme === item.id 
                      ? "bg-primary/5 border-primary text-primary" 
                      : "bg-background-tertiary border-transparent text-muted-foreground hover:border-border"
                  }`}
                >
                  <item.icon className="h-6 w-6" />
                  <span className="text-sm font-bold">{item.label}</span>
                </button>
              ))}
            </div>
          </div>
        </section>

        {/* About */}
        <section className="bg-background-secondary border border-border rounded-3xl overflow-hidden">
          <div className="p-8">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <div className="p-3 bg-emerald-500/10 text-emerald-500 rounded-2xl">
                  <Shield className="h-8 w-8" />
                </div>
                <div>
                  <h2 className="text-2xl font-black text-foreground tracking-tighter">ChronoDesk</h2>
                  <p className="text-sm text-muted-foreground font-medium">Version {appVersion} (Stable Alpha)</p>
                </div>
              </div>
              <div className="flex items-center gap-4">
                <button className="p-3 bg-background-tertiary text-muted-foreground hover:text-foreground rounded-xl transition-all">
                  <GitBranch className="h-5 w-5" />
                </button>
                <button className="p-3 bg-background-tertiary text-muted-foreground hover:text-foreground rounded-xl transition-all">
                  <Info className="h-5 w-5" />
                </button>
              </div>
            </div>
            <div className="mt-8 pt-8 border-t border-border flex items-center justify-between text-[10px] font-bold text-muted-foreground uppercase tracking-widest">
              <span>© 2026 ChronoDesk Labs</span>
              <div className="flex items-center gap-4">
                <a href="#" className="hover:text-primary transition-colors">Privacy Policy</a>
                <a href="#" className="hover:text-primary transition-colors">Terms of Service</a>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
