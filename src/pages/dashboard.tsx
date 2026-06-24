import { createResource, createSignal, Show, For, onMount, onCleanup } from "solid-js";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { listFolders, listConflicts, getRecentActivity, triggerSyncAll } from "~/lib/tauri";
import { listen } from "@tauri-apps/api/event";
import { Play } from "lucide-solid";
import { formatDateShort } from "~/lib/utils";

export default function Dashboard() {
  const [folders, { refetch: refetchFolders }] = createResource(listFolders);
  const [conflicts] = createResource(() => listConflicts(false));
  const [activity, { refetch: refetchActivity }] = createResource(() => getRecentActivity(10));

  // Auto-refresh on background sync events
  onMount(() => {
    let unlisten: (() => void) | undefined;
    listen<string>("sync-status-changed", () => {
      refetchFolders();
      refetchActivity();
    }).then((fn) => { unlisten = fn; });
    onCleanup(() => { unlisten?.(); });
  });

  const [syncingAll, setSyncingAll] = createSignal(false);

  const handleSyncAll = async () => {
    if (syncingAll()) return;
    setSyncingAll(true);
    try {
      await triggerSyncAll();
    } catch (e: any) {
      console.error("Sync All failed:", e?.message || e);
    } finally {
      // Brief cooldown to prevent rapid double-clicks; the actual sync
      // progress is tracked via sync-status-changed events.
      setTimeout(() => setSyncingAll(false), 1500);
    }
  };

  const stats = () => {
    const f = folders() || [];
    return {
      total: f.length,
      syncing: f.filter((x) => x.status === "syncing").length,
      errors: f.filter((x) => x.status === "error").length,
      conflicts: (conflicts() || []).length,
    };
  };

  return (
    <div class="flex flex-col h-full gap-6">
      {/* Page header */}
      <div class="flex items-center justify-between shrink-0">
        <div>
          <h1 class="text-2xl font-bold tracking-tight text-zinc-900">
            Dashboard
          </h1>
          <p class="text-sm text-zinc-500">Overview of your sync status</p>
        </div>
        <Button
          variant="primary"
          size="sm"
          onClick={handleSyncAll}
          disabled={syncingAll()}
        >
          <Play class="w-3.5 h-3.5" />
          {syncingAll() ? "Syncing..." : "Sync All"}
        </Button>
      </div>

      {/* Stats */}
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4 shrink-0">
        <Card>
          <CardHeader class="pb-2">
            <CardTitle class="text-xs font-semibold uppercase tracking-wider text-zinc-500">
              Total Folders
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold tracking-tight text-zinc-900">
              {stats().total}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="pb-2">
            <CardTitle class="text-xs font-semibold uppercase tracking-wider text-zinc-500">
              Syncing
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold tracking-tight text-amber-600">
              {stats().syncing}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="pb-2">
            <CardTitle class="text-xs font-semibold uppercase tracking-wider text-zinc-500">
              Errors
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold tracking-tight text-red-600">
              {stats().errors}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="pb-2">
            <CardTitle class="text-xs font-semibold uppercase tracking-wider text-zinc-500">
              Conflicts
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold tracking-tight text-orange-600">
              {stats().conflicts}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Recent Activity */}
      <Card class="min-h-0 flex-1 flex flex-col overflow-hidden">
        <CardHeader class="shrink-0">
          <CardTitle>Recent Activity</CardTitle>
        </CardHeader>
        <CardContent class="overflow-y-auto scrollbar-hidden min-h-0">
          <Show
            when={activity() && activity()!.length > 0}
            fallback={
              <p class="text-sm text-zinc-500">No recent activity</p>
            }
          >
            <div class="divide-y divide-zinc-100">
              <For each={activity()}>
                {(log) => {
                  // Look up folder name from folder_id
                  const folder = () => (folders() || []).find((f) => f.id === log.folder_id);
                  const folderName = () => {
                    const f = folder();
                    if (!f) return log.folder_id.slice(0, 8);
                    const parts = f.local_path.replace(/\/+$/, "").split("/");
                    return parts[parts.length - 1] || f.local_path;
                  };

                  return (
                    <div class="flex items-center py-2.5 gap-3 first:pt-0 last:pb-0">
                      <Badge
                        variant={
                          log.status === "success"
                            ? "success"
                            : log.status === "error"
                            ? "error"
                            : "warning"
                        }
                        class="shrink-0"
                      >
                        {log.status}
                      </Badge>
                      <span class="text-sm font-medium text-zinc-800 shrink-0 min-w-[80px] max-w-[140px] truncate">
                        {folderName()}
                      </span>
                      <span class="text-xs text-zinc-500 flex-1 min-w-0 truncate">
                        {log.message || "—"}
                      </span>
                      <span class="text-xs text-zinc-400 shrink-0">
                        {formatDateShort(log.timestamp)}
                      </span>
                    </div>
                  );
                }}
              </For>
            </div>
          </Show>
        </CardContent>
      </Card>
    </div>
  );
}
