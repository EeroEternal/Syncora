import { createResource, Show, For } from "solid-js";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { listConflicts, resolveConflict } from "~/lib/tauri";
import { formatDate } from "~/lib/utils";
import { CheckCircle, AlertTriangle } from "lucide-solid";

export default function Conflicts() {
  const [conflicts, { refetch }] = createResource(() => listConflicts(false));

  const handleResolve = async (
    id: string,
    resolution: "keep_local" | "keep_remote" | "keep_both"
  ) => {
    await resolveConflict(id, resolution);
    refetch();
  };

  return (
    <div class="space-y-6">
      <div>
        <h1 class="text-2xl font-bold tracking-tight text-zinc-900">
          Conflicts
        </h1>
        <p class="text-sm text-zinc-500">Resolve file sync conflicts</p>
      </div>

      <Show
        when={conflicts() && conflicts()!.length > 0}
        fallback={
          <div class="flex flex-col items-center justify-center rounded-lg border border-dashed border-zinc-300 bg-zinc-50/50 py-12 px-6">
            <CheckCircle class="w-8 h-8 text-zinc-400 mb-3" />
            <p class="text-sm text-zinc-500">No conflicts to resolve</p>
          </div>
        }
      >
        <div class="bg-white border border-zinc-200 rounded-lg shadow-sm divide-y divide-zinc-200">
          <For each={conflicts()}>
            {(conflict) => (
              <div class="px-5 py-4">
                <div class="flex items-start justify-between gap-4">
                  <div class="min-w-0">
                    <div class="flex items-center gap-2">
                      <AlertTriangle class="w-4 h-4 text-amber-500 shrink-0" />
                      <span class="font-medium text-sm text-zinc-900 truncate">
                        {conflict.file_path}
                      </span>
                    </div>
                    <div class="text-xs text-zinc-500 mt-2 space-y-0.5">
                      <div>Detected: {formatDate(conflict.detected_at)}</div>
                      <Show when={conflict.local_version}>
                        <div>Local: {conflict.local_version}</div>
                      </Show>
                      <Show when={conflict.remote_version}>
                        <div>Remote: {conflict.remote_version}</div>
                      </Show>
                    </div>
                  </div>
                  <div class="flex gap-2 shrink-0">
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => handleResolve(conflict.id, "keep_local")}
                    >
                      Keep Local
                    </Button>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => handleResolve(conflict.id, "keep_remote")}
                    >
                      Keep Remote
                    </Button>
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={() => handleResolve(conflict.id, "keep_both")}
                    >
                      Keep Both
                    </Button>
                  </div>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
