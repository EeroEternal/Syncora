import { createResource, createSignal, Show, For } from "solid-js";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import {
  listFolders,
  addFolder,
  deleteFolder,
  triggerSync,
  releaseFolder,
} from "~/lib/tauri";
import type { Folder } from "~/lib/tauri";
import { formatDate } from "~/lib/utils";

export default function Folders() {
  const [folders, { refetch }] = createResource(listFolders);
  const [showAdd, setShowAdd] = createSignal(false);
  const [newPath, setNewPath] = createSignal("");
  const [newPrefix, setNewPrefix] = createSignal("");

  const handleAdd = async () => {
    if (newPath() && newPrefix()) {
      await addFolder(newPath(), newPrefix());
      setNewPath("");
      setNewPrefix("");
      setShowAdd(false);
      refetch();
    }
  };

  const handleDelete = async (id: string) => {
    await deleteFolder(id);
    refetch();
  };

  const handleSync = async (id: string) => {
    await triggerSync(id);
    refetch();
  };

  const handleRelease = async (id: string) => {
    await releaseFolder(id);
    refetch();
  };

  const statusVariant = (status: Folder["status"]) => {
    switch (status) {
      case "synced":
        return "success" as const;
      case "syncing":
        return "warning" as const;
      case "error":
        return "destructive" as const;
      case "released":
        return "outline" as const;
      default:
        return "default" as const;
    }
  };

  return (
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold">Folders</h1>
          <p class="text-sm text-[hsl(var(--muted-foreground))]">
            Manage your synced folders
          </p>
        </div>
        <Button onClick={() => setShowAdd(!showAdd())}>
          {showAdd() ? "Cancel" : "Add Folder"}
        </Button>
      </div>

      {/* Add Folder Form */}
      <Show when={showAdd()}>
        <Card>
          <CardContent class="pt-6">
            <div class="flex gap-4 items-end">
              <div class="flex-1">
                <Input
                  label="Local Path"
                  placeholder="/path/to/folder"
                  value={newPath()}
                  onInput={(e) => setNewPath(e.currentTarget.value)}
                />
              </div>
              <div class="flex-1">
                <Input
                  label="Remote Prefix"
                  placeholder="documents/work"
                  value={newPrefix()}
                  onInput={(e) => setNewPrefix(e.currentTarget.value)}
                />
              </div>
              <Button onClick={handleAdd}>Add</Button>
            </div>
          </CardContent>
        </Card>
      </Show>

      {/* Folder List */}
      <Show
        when={folders() && folders()!.length > 0}
        fallback={
          <Card>
            <CardContent class="py-12 text-center">
              <p class="text-[hsl(var(--muted-foreground))]">
                No folders added yet. Click "Add Folder" to get started.
              </p>
            </CardContent>
          </Card>
        }
      >
        <div class="space-y-3">
          <For each={folders()}>
            {(folder) => (
              <Card>
                <CardContent class="flex items-center justify-between py-4 px-6">
                  <div class="flex items-center gap-4">
                    <span class="text-2xl">📁</span>
                    <div>
                      <div class="flex items-center gap-2">
                        <span class="font-medium text-sm">
                          {folder.local_path}
                        </span>
                        <Badge variant={statusVariant(folder.status)}>
                          {folder.status}
                        </Badge>
                        <Show when={folder.mode === "cloud_only"}>
                          <Badge variant="outline">☁️ Cloud Only</Badge>
                        </Show>
                      </div>
                      <div class="text-xs text-[hsl(var(--muted-foreground))] mt-1">
                        Remote: {folder.remote_prefix} • Last sync:{" "}
                        {formatDate(folder.last_sync_at)}
                      </div>
                    </div>
                  </div>
                  <div class="flex items-center gap-2">
                    <Show when={folder.mode !== "cloud_only"}>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleSync(folder.id)}
                        disabled={folder.status === "syncing"}
                      >
                        Sync
                      </Button>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => handleRelease(folder.id)}
                      >
                        Release
                      </Button>
                    </Show>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDelete(folder.id)}
                    >
                      🗑️
                    </Button>
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
