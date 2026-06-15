import { createResource, createSignal, Show, For } from "solid-js";
import { Badge } from "~/components/ui/badge";
import { getLogs } from "~/lib/tauri";
import { formatDate, formatDuration } from "~/lib/utils";
import { FileText } from "lucide-solid";
import {
  tableShellClass,
  tableHeadRowClass,
  tableBodyDivideClass,
  tableBodyRowClass,
  tableHeadCellClass,
  tableBodyCellClass,
  emptyStateClass,
} from "~/lib/tokens";

export default function Logs() {
  const [limit, setLimit] = createSignal(50);
  const [logs] = createResource(limit, (l) => getLogs(undefined, l));

  return (
    <div class="space-y-6">
      <div>
        <h1 class="text-2xl font-bold tracking-tight text-zinc-900">
          Sync Logs
        </h1>
        <p class="text-sm text-zinc-500">History of all sync operations</p>
      </div>

      {/* Table */}
      <div class={tableShellClass}>
        <Show
          when={logs() && logs()!.length > 0}
          fallback={
            <div class={emptyStateClass}>
              <FileText class="w-8 h-8 text-zinc-400 mb-3" />
              <p class="text-sm text-zinc-500">No logs yet</p>
            </div>
          }
        >
          <table class="w-full">
            <thead>
              <tr class={tableHeadRowClass}>
                <th class={tableHeadCellClass}>Time</th>
                <th class={tableHeadCellClass}>Action</th>
                <th class={tableHeadCellClass}>Status</th>
                <th class={tableHeadCellClass}>Message</th>
                <th class={tableHeadCellClass}>Duration</th>
              </tr>
            </thead>
            <tbody class={tableBodyDivideClass}>
              <For each={logs()}>
                {(log) => (
                  <tr class={tableBodyRowClass}>
                    <td class={`${tableBodyCellClass} text-xs text-zinc-500`}>
                      {formatDate(log.timestamp)}
                    </td>
                    <td class={tableBodyCellClass}>{log.action}</td>
                    <td class={tableBodyCellClass}>
                      <Badge
                        variant={
                          log.status === "success"
                            ? "success"
                            : log.status === "error"
                            ? "error"
                            : "warning"
                        }
                      >
                        {log.status}
                      </Badge>
                    </td>
                    <td class={`${tableBodyCellClass} text-xs max-w-xs truncate`}>
                      {log.message || "—"}
                    </td>
                    <td class={`${tableBodyCellClass} text-xs font-mono`}>
                      {log.duration_ms ? formatDuration(log.duration_ms) : "—"}
                    </td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </Show>
      </div>
    </div>
  );
}
