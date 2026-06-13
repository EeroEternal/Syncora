import { createResource, Show, For } from "solid-js";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { listFolders, listConflicts, getRecentActivity, triggerSyncAll } from "~/lib/tauri";
import type { Folder, Conflict, SyncLog } from "~/lib/tauri";
import { formatDate } from "~/lib/utils";

export default function Dashboard() {
  const [folders] = createResource(listFolders);
  const [conflicts] = createResource(() => listConflicts(false));
  const [activity] = createResource(() => getRecentActivity(10));

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
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold">Dashboard</h1>
          <p class="text-sm text-[hsl(var(--muted-foreground))]">
            Overview of your sync status
          </p>
        </div>
        <Button onClick={() => triggerSyncAll()}>Sync All</Button>
      </div>

      {/* Status Cards */}
      <div class="grid grid-cols-4 gap-4">
        <Card>
          <CardHeader class="pb-2">
            <CardTitle class="text-sm font-medium text-[hsl(var(--muted-foreground))]">
              Total Folders
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold">{stats().total}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="pb-2">
            <CardTitle class="text-sm font-medium text-[hsl(var(--muted-foreground))]">
              Syncing
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold text-[hsl(var(--warning))]">
              {stats().syncing}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="pb-2">
            <CardTitle class="text-sm font-medium text-[hsl(var(--muted-foreground))]">
              Errors
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold text-[hsl(var(--destructive))]">
              {stats().errors}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="pb-2">
            <CardTitle class="text-sm font-medium text-[hsl(var(--muted-foreground))]">
              Conflicts
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold text-[hsl(var(--conflict))]">
              {stats().conflicts}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Recent Activity */}
      <Card>
        <CardHeader>
          <CardTitle>Recent Activity</CardTitle>
        </CardHeader>
        <CardContent>
          <Show
            when={activity() && activity()!.length > 0}
            fallback={
              <p class="text-sm text-[hsl(var(--muted-foreground))]">
                No recent activity
              </p>
            }
          >
            <div class="space-y-3">
              <For each={activity()}>
                {(log) => (
                  <div class="flex items-center justify-between border-b border-[hsl(var(--border))] pb-2 last:border-0">
                    <div class="flex items-center gap-3">
                      <Badge
                        variant={
                          log.status === "success"
                            ? "success"
                            : log.status === "error"
                            ? "destructive"
                            : "warning"
                        }
                      >
                        {log.status}
                      </Badge>
                      <span class="text-sm">{log.action}</span>
                    </div>
                    <span class="text-xs text-[hsl(var(--muted-foreground))]">
                      {formatDate(log.timestamp)}
                    </span>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </CardContent>
      </Card>
    </div>
  );
}
