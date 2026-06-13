import { createResource, Show, For } from "solid-js";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { listConflicts, resolveConflict } from "~/lib/tauri";
import { formatDate } from "~/lib/utils";

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
        <h1 class="text-2xl font-bold">Conflicts</h1>
        <p class="text-sm text-[hsl(var(--muted-foreground))]">
          Resolve file sync conflicts
        </p>
      </div>

      <Show
        when={conflicts() && conflicts()!.length > 0}
        fallback={
          <Card>
            <CardContent class="py-12 text-center">
              <div class="text-4xl mb-3">✅</div>
              <p class="text-[hsl(var(--muted-foreground))]">
                No conflicts to resolve
              </p>
            </CardContent>
          </Card>
        }
      >
        <div class="space-y-3">
          <For each={conflicts()}>
            {(conflict) => (
              <Card>
                <CardContent class="py-4 px-6">
                  <div class="flex items-start justify-between">
                    <div>
                      <div class="flex items-center gap-2">
                        <Badge variant="conflict">Conflict</Badge>
                        <span class="font-medium text-sm">
                          {conflict.file_path}
                        </span>
                      </div>
                      <div class="text-xs text-[hsl(var(--muted-foreground))] mt-2 space-y-1">
                        <div>Detected: {formatDate(conflict.detected_at)}</div>
                        <Show when={conflict.local_version}>
                          <div>Local: {conflict.local_version}</div>
                        </Show>
                        <Show when={conflict.remote_version}>
                          <div>Remote: {conflict.remote_version}</div>
                        </Show>
                      </div>
                    </div>
                    <div class="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleResolve(conflict.id, "keep_local")}
                      >
                        Keep Local
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() =>
                          handleResolve(conflict.id, "keep_remote")
                        }
                      >
                        Keep Remote
                      </Button>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => handleResolve(conflict.id, "keep_both")}
                      >
                        Keep Both
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
