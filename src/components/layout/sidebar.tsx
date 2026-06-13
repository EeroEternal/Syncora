import { A, useLocation } from "@solidjs/router";
import { cn } from "~/lib/utils";

const navItems = [
  { path: "/", label: "Dashboard", icon: "📊" },
  { path: "/folders", label: "Folders", icon: "📁" },
  { path: "/conflicts", label: "Conflicts", icon: "⚠️" },
  { path: "/logs", label: "Logs", icon: "📋" },
  { path: "/settings", label: "Settings", icon: "⚙️" },
];

export default function Sidebar() {
  const location = useLocation();

  return (
    <aside class="flex h-full w-56 flex-col border-r border-[hsl(var(--border))] bg-[hsl(var(--card))]">
      <div class="flex h-14 items-center gap-2 border-b border-[hsl(var(--border))] px-4">
        <span class="text-xl font-bold text-[hsl(var(--primary))]">Syncora</span>
      </div>
      <nav class="flex-1 space-y-1 p-3">
        {navItems.map((item) => (
          <A
            href={item.path}
            class={cn(
              "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
              location.pathname === item.path
                ? "bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]"
                : "text-[hsl(var(--muted-foreground))] hover:bg-[hsl(var(--accent))] hover:text-[hsl(var(--accent-foreground))]"
            )}
          >
            <span class="text-base">{item.icon}</span>
            <span>{item.label}</span>
          </A>
        ))}
      </nav>
      <div class="border-t border-[hsl(var(--border))] p-3">
        <div class="flex items-center gap-2 rounded-md px-3 py-2">
          <div class="h-2 w-2 rounded-full bg-[hsl(var(--success))]"></div>
          <span class="text-xs text-[hsl(var(--muted-foreground))]">All synced</span>
        </div>
      </div>
    </aside>
  );
}
