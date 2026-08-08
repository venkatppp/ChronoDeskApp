import { Outlet } from "react-router-dom";
import { Sidebar } from "@/components/navigation/Sidebar";
import { Topbar } from "@/components/navigation/Topbar";

export function AppLayout() {
  return (
    <div className="flex h-screen w-screen overflow-hidden bg-(--color-background) text-(--color-foreground)">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar />
        <main className="flex-1 overflow-y-auto animate-(--animate-fade-in)">
          <Outlet />
        </main>
      </div>
    </div>
  );
}