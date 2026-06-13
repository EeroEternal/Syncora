import { createResource, createSignal, Show, For } from "solid-js";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";
import { getLogs } from "~/lib/tauri";
import { formatDate, formatDuration } from "~/lib/utils";

export default function Logs() {
  const [limit, setLimit] = createSignal(50);
  const [logs] = createResource(limit, (l) => getLogs(undefined, l));

  return (
    <div class="space-y-6">
      <div>
        <h1 class="text-2xl font-bold">Sync Logs</h1>
        <p class="text-sm text-[hsl(var(--muted-foreground))]">
          History of all sync operations
        </p>
      </div>

      <Card>
        <CardContent class="p-0">
          <Show
            when={logs() && logs()!.length > 0}
            fallback={
              <div class="py-12 text-center">
                <p class="text-[hsl(var(--muted-foreground))]">No logs yet</p>
              </div>
            }
          >
            <div class="overflow-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b border-[hsl(var(--border))]">
                    <th class="px-4 py-3 text-left font-medium text-[hsl(var(--muted-foreground))]">
                      Time
                    </th>
                    <th class="px-4 py-3 text-left font-medium text-[hsl(var(--muted-foreground))]">
                      Action
                    </th>
                    <th class="px-4 py-3 text-left font-medium text-[hsl(var(--muted-foreground))]">
                      Status
                    </th>
                    <th class="px-4 py-3 text-left font-medium text-[hsl(var(--muted-foreground))]">
                      Message
                    </th>
                    <th class="px-4 py-3 text-left font-medium text-[hsl(var(--muted-foreground))]">
                      Duration
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <For each={logs()}>
                    {(log) => (
                      <tr class="border-b border-[hsl(var(--border))] last:border-0">
                        <td class="px-4 py-3 text-xs text-[hsl(var(--muted-foreground))]">
                          {formatDate(log.timestamp)}
                        </td>
                        <td class="px-4 py-3">{log.action}</td>
                        <td class="px-4 py-3">
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
                        </td>
                        <td class="px-4 py-3 text-xs max-w-xs truncate">
                          {log.message || "—"}
                        </td>
                        <td class="px-4 py-3 text-xs">
                          {log.duration_ms ? formatDuration(log.duration_ms) : "—"}
                        </td>
                      </tr>
                    )}
                  </For>
                </tbody>
              </table>
            </div>
          </Show>
        </CardContent>
      </Card>
    </div>
  );
}
