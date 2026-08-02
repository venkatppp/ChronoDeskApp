import { HashRouter, Routes, Route } from "react-router-dom";
import { AppLayout } from "@/layouts/AppLayout";
import { ThemeProvider } from "@/contexts/ThemeContext";
import { DashboardPage } from "@/pages/DashboardPage";
import { WorkspacesPage } from "@/pages/WorkspacesPage";
import { TimelinePage } from "@/pages/TimelinePage";
import { GraphPage } from "@/pages/GraphPage";
import { SearchPage } from "@/pages/SearchPage";
import { AnalyticsPage } from "@/pages/AnalyticsPage";
import { SettingsPage } from "@/pages/SettingsPage";
import LearningPage from "@/pages/LearningPage";
import { AutonomousPage } from "@/pages/AutonomousPage";
import { CopilotPage } from "@/pages/CopilotPage";
import { ExecutionPage } from "@/pages/ExecutionPage";
import { MemoryPage } from "@/pages/MemoryPage";

/**
 * `HashRouter` is used rather than `BrowserRouter` because Tauri serves the
 * frontend from a custom asset protocol (not a real HTTP server with
 * history-API fallback). Hash-based routing avoids 404s on refresh/deep
 * links in the packaged desktop build.
 */
export function App() {
  return (
    <ThemeProvider>
      <HashRouter>
        <Routes>
          <Route element={<AppLayout />}>
            <Route index element={<DashboardPage />} />
            <Route path="workspaces" element={<WorkspacesPage />} />
            <Route path="timeline" element={<TimelinePage />} />
            <Route path="graph" element={<GraphPage />} />
            <Route path="search" element={<SearchPage />} />
            <Route path="analytics" element={<AnalyticsPage />} />
            <Route path="learning" element={<LearningPage />} />
            <Route path="copilot" element={<CopilotPage />} />
            <Route path="executions" element={<ExecutionPage />} />
            <Route path="autonomous" element={<AutonomousPage />} />
            <Route path="memory" element={<MemoryPage />} />
            <Route path="settings" element={<SettingsPage />} />
          </Route>
        </Routes>
      </HashRouter>
    </ThemeProvider>
  );
}
