import { A, useLocation } from "@solidjs/router";
import { cn } from "~/lib/utils";
import {
  LayoutDashboard,
  FolderSync,
  AlertTriangle,
  FileText,
  Settings,
} from "lucide-solid";

const navItems = [
  { path: "/", label: "Home", icon: LayoutDashboard },
  { path: "/folders", label: "Folders", icon: FolderSync },
  { path: "/conflicts", label: "Conflicts", icon: AlertTriangle },
  { path: "/logs", label: "Logs", icon: FileText },
  { path: "/settings", label: "Settings", icon: Settings },
];

export default function BottomNav() {
  const location = useLocation();

  return (
    <nav class="flex md:hidden fixed bottom-0 left-0 right-0 h-14 bg-white border-t border-zinc-200 z-50">
      {navItems.map((item) => {
        const isActive = () => location.pathname === item.path;
        const Icon = item.icon;
        return (
          <A
            href={item.path}
            class={cn(
              "flex flex-1 flex-col items-center justify-center gap-0.5 text-[10px] font-medium transition-colors",
              isActive()
                ? "text-zinc-900"
                : "text-zinc-400 hover:text-zinc-600"
            )}
          >
            <Icon class="w-5 h-5 shrink-0" />
            <span>{item.label}</span>
          </A>
        );
      })}
    </nav>
  );
}
