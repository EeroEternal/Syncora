import { A, useLocation, useNavigate } from "@solidjs/router";
import { createResource } from "solid-js";
import { cn } from "~/lib/utils";
import { getAuthStatus } from "~/lib/tauri";
import {
  LayoutDashboard,
  FolderSync,
  AlertTriangle,
  FileText,
  Settings,
} from "lucide-solid";

const navItems = [
  { path: "/", label: "Dashboard", icon: LayoutDashboard },
  { path: "/folders", label: "Folders", icon: FolderSync },
  { path: "/conflicts", label: "Conflicts", icon: AlertTriangle },
  { path: "/logs", label: "Logs", icon: FileText },
  { path: "/settings", label: "Settings", icon: Settings },
];

export default function Sidebar() {
  const location = useLocation();
  const navigate = useNavigate();
  const [authStatus] = createResource(getAuthStatus);

  return (
    <aside class="flex h-full w-56 flex-col border-r border-zinc-200 bg-white">
      {/* Navigation */}
      <nav class="flex-1 space-y-1 p-3">
        {navItems.map((item) => {
          const isActive = () => location.pathname === item.path;
          const Icon = item.icon;
          return (
            <A
              href={item.path}
              class={cn(
                "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                isActive()
                  ? "bg-zinc-100 text-zinc-900"
                  : "text-zinc-600 hover:bg-zinc-50 hover:text-zinc-900"
              )}
            >
              <Icon class="w-4 h-4 shrink-0" />
              <span>{item.label}</span>
            </A>
          );
        })}
      </nav>

      {/* User section */}
      <div class="border-t border-zinc-200 p-3">
        {authStatus()?.logged_in ? (
          <div class="flex items-center gap-2 rounded-md px-3 py-2">
            <div class="h-2 w-2 rounded-full bg-emerald-500 shrink-0" />
            <span class="text-xs text-zinc-500 truncate">
              {authStatus()?.user?.email}
            </span>
          </div>
        ) : (
          <button
            class="flex items-center gap-2 rounded-md px-3 py-2 text-xs text-zinc-900 hover:underline font-medium"
            onClick={() => navigate("/login")}
          >
            Sign In
          </button>
        )}
      </div>
    </aside>
  );
}
