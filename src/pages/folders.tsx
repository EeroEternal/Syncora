import { createResource, createSignal, Show, For, onMount, onCleanup } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { Badge } from "~/components/ui/badge";
import { Button, IconButton } from "~/components/ui/button";
import { Dialog, DialogBody, DialogFooter } from "~/components/ui/dialog";
import {
  listFolders,
  addFolder,
  deleteFolder,
  releaseFolder,
  cancelSync,
  triggerSync,
  openFolder,
} from "~/lib/tauri";
import type { Folder } from "~/lib/tauri";
import { formatDate, formatDateShort } from "~/lib/utils";
import {
  Plus,
  CloudOff,
  Trash2,
  FolderOpen,
  AlertTriangle,
  Info,
  Check,
  X,
  Loader2,
  ExternalLink,
} from "lucide-solid";

type ConflictKind = "server" | "local" | null;

export default function Folders() {
  const [folders, { refetch }] = createResource(listFolders);
  const [showDialog, setShowDialog] = createSignal(false);
  const [selectedPath, setSelectedPath] = createSignal("");
  const [loadingAction, setLoadingAction] = createSignal<string | null>(null);
  const [doneAction, setDoneAction] = createSignal<string | null>(null);
  const [error, setError] = createSignal("");
  const [openError, setOpenError] = createSignal("");
  const [conflict, setConflict] = createSignal<ConflictKind>(null);
  const [syncingId, setSyncingId] = createSignal<string | null>(null);

  // Per-folder live progress data, keyed by folder_id
  type ProgressData = {
    percent?: number;
    current_file?: string;
    event: "progress" | "file" | "done" | "cancelled";
  };
  const [progress, setProgress] = createSignal<Record<string, ProgressData>>({});
  const [cancelling, setCancelling] = createSignal<Record<string, boolean>>({});

  // Listen for sync-status-changed events from backend to auto-refresh
  onMount(() => {
    const unlisteners: Array<() => void> = [];

    listen<string>("sync-status-changed", () => {
      refetch();
    }).then((fn) => { unlisteners.push(fn); });

    listen<{ folder_id: string; event: string; percent?: number; file_path?: string; raw?: string }>(
      "sync-progress",
      (ev) => {
        const { folder_id, event } = ev.payload;
        if (event === "progress") {
          setProgress((p) => ({
            ...p,
            [folder_id]: {
              percent: ev.payload.percent,
              current_file: p[folder_id]?.current_file,
              event: "progress",
            },
          }));
        } else if (event === "file") {
          setProgress((p) => ({
            ...p,
            [folder_id]: {
              percent: p[folder_id]?.percent,
              current_file: ev.payload.file_path,
              event: "file",
            },
          }));
        } else if (event === "done" || event === "cancelled") {
          setProgress((p) => {
            const next = { ...p };
            delete next[folder_id];
            return next;
          });
          setCancelling((c) => {
            const next = { ...c };
            delete next[folder_id];
            return next;
          });
        }
      }
    ).then((fn) => { unlisteners.push(fn); });

    onCleanup(() => { unlisteners.forEach((f) => f()); });
  });

  const isBusy = () => loadingAction() !== null || doneAction() !== null;

  const openAddDialog = () => {
    setSelectedPath("");
    setError("");
    setConflict(null);
    setDoneAction(null);
    setShowDialog(true);
  };

  const closeDialog = () => {
    if (!isBusy()) {
      setShowDialog(false);
      setSelectedPath("");
      setError("");
      setConflict(null);
    }
  };

  const pickDirectory = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select folder to sync",
    });
    if (selected) {
      setSelectedPath(selected as string);
      setError("");
      setConflict(null);
    }
  };

  const doAdd = async (mode: "create" | "connect" | "replace") => {
    const path = selectedPath();
    if (!path || isBusy()) return;

    setLoadingAction(mode);
    setError("");
    if (mode !== "connect" && mode !== "replace") {
      setConflict(null);
    }

    try {
      const folder = await addFolder(path, mode);
      // Clear loading, show done feedback
      setLoadingAction(null);
      setDoneAction(mode);
      await new Promise((r) => setTimeout(r, 600));
      setDoneAction(null);
      setSelectedPath("");
      setConflict(null);
      setShowDialog(false);
      refetch();
      // Trigger immediate sync for the new folder.
      // Don't silently swallow errors — show feedback so the user knows
      // if the sync failed to start.
      triggerSync(folder.id).catch((e: any) => {
        const msg = e?.message || String(e) || "Failed to start sync";
        console.error("triggerSync error after add:", msg);
        setOpenError(`Sync failed to start: ${msg}`);
        setTimeout(() => setOpenError(""), 5000);
        refetch();
      });
      // Delayed refetch to pick up the "syncing" status set by the backend.
      setTimeout(() => refetch(), 1000);
    } catch (e: any) {
      const msg = e?.message || String(e) || "Failed to add folder";
      if (msg.includes("already") || msg.includes("already exists")) {
        if (msg.includes("already synced on the server")) {
          setConflict("server");
        } else {
          setConflict("local");
        }
      } else {
        setError(msg);
      }
    } finally {
      setLoadingAction(null);
    }
  };

  const handleDelete = async (id: string) => {
    setSyncingId(`delete:${id}`);
    try {
      await deleteFolder(id);
      refetch();
    } finally {
      setSyncingId(null);
    }
  };

  const handleRelease = async (id: string) => {
    setSyncingId(`release:${id}`);
    try {
      await releaseFolder(id);
      refetch();
    } finally {
      setSyncingId(null);
    }
  };

  const handleOpenFolder = async (path: string) => {
    console.log("[Folders] openFolder clicked, path =", JSON.stringify(path));
    try {
      await openFolder(path);
      console.log("[Folders] openFolder succeeded for", path);
    } catch (e: any) {
      const msg = e?.message || String(e) || "Failed to open folder";
      console.error("openFolder error:", msg, "| path:", path);
      setOpenError(msg);
      setTimeout(() => setOpenError(""), 4000);
    }
  };

  const handleCancel = async (id: string) => {
    setCancelling((c) => ({ ...c, [id]: true }));
    try {
      await cancelSync(id);
      // Progress event "cancelled" will clear the progress UI
    } catch (e) {
      setCancelling((c) => {
        const next = { ...c };
        delete next[id];
        return next;
      });
    }
  };

  const statusVariant = (status: Folder["status"]) => {
    switch (status) {
      case "synced":
        return "success" as const;
      case "syncing":
        return "warning" as const;
      case "error":
        return "error" as const;
      case "released":
        return "outline" as const;
      default:
        return "default" as const;
    }
  };

  const folderName = () => {
    const p = selectedPath();
    if (!p) return "";
    const parts = p.replace(/\/+$/, "").split("/");
    return parts[parts.length - 1] || "";
  };

  const getFolderName = (path: string) => {
    const parts = path.replace(/\/+$/, "").split("/");
    return parts[parts.length - 1] || path;
  };

  const getFolderDir = (path: string) => {
    const parts = path.replace(/\/+$/, "").split("/");
    return parts.slice(0, -1).join("/") || "/";
  };

  return (
    <div class="space-y-6">
      {/* Page header */}
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold tracking-tight text-zinc-900">
            Folders
          </h1>
          <p class="text-sm text-zinc-500">Manage your synced folders</p>
        </div>
        <Button variant="primary" size="sm" onClick={openAddDialog}>
          <Plus class="w-3.5 h-3.5" />
          Add Folder
        </Button>
      </div>

      {/* Add Folder Dialog */}
      <Dialog
        open={showDialog()}
        onClose={closeDialog}
        title="Add Sync Folder"
        subtitle="Choose a local folder to sync with the cloud. Remote path is generated automatically."
        structured
      >
        <DialogBody>
          {/* Directory picker */}
          <Show
            when={selectedPath()}
            fallback={
              <button
                class="w-full border-2 border-dashed border-zinc-300 rounded-lg py-10 px-4 text-center hover:border-zinc-400 hover:bg-zinc-50 transition-colors cursor-pointer"
                onClick={pickDirectory}
              >
                <FolderOpen class="w-8 h-8 mx-auto mb-3 text-zinc-400" />
                <div class="text-sm font-medium text-zinc-900">
                  Click to select a folder
                </div>
                <div class="text-xs text-zinc-500 mt-1">
                  Browse your computer to pick a folder to sync
                </div>
              </button>
            }
          >
            <div class="flex items-center gap-3 bg-zinc-50 rounded-lg p-4 border border-zinc-200">
              <FolderOpen class="w-5 h-5 text-zinc-500 shrink-0" />
              <div class="flex-1 min-w-0">
                <div class="text-sm font-medium text-zinc-900 truncate">
                  {folderName()}
                </div>
                <div class="text-xs text-zinc-500 truncate">
                  {selectedPath()}
                </div>
              </div>
              <Button
                variant="secondary"
                size="sm"
                onClick={pickDirectory}
                class={isBusy() ? "pointer-events-none" : undefined}
              >
                Change
              </Button>
            </div>
          </Show>

          {/* Conflict confirmation */}
          <Show when={conflict()}>
            {(kind) => (
              <div class="mt-4 rounded-md border border-amber-200 bg-amber-50 px-3 py-2.5">
                <div class="flex items-start gap-2">
                  <AlertTriangle class="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
                  <div class="flex-1 min-w-0">
                    <div class="text-sm font-medium text-amber-900">
                      {kind() === "server"
                        ? `A folder named "${folderName()}" already exists on the server.`
                        : "This folder is already being synced locally."}
                    </div>
                    <div class="text-xs text-amber-700 mt-0.5">
                      Choose how to proceed below.
                    </div>
                  </div>
                </div>
              </div>
            )}
          </Show>
        </DialogBody>

        <DialogFooter>
          <Button variant="secondary" size="sm" onClick={closeDialog} class={isBusy() ? "pointer-events-none" : undefined}>
            Cancel
          </Button>

          <Show
            when={conflict()}
            fallback={
              <Button
                variant="primary"
                size="sm"
                loading={loadingAction() === "create"}
                disabled={!selectedPath() && doneAction() !== "create"}
                class={doneAction() === "create" ? "min-w-[120px] !bg-emerald-600" : "min-w-[120px]"}
                onClick={() => doAdd("create")}
              >
                {loadingAction() === "create"
                  ? "Starting..."
                  : doneAction() === "create"
                  ? <><Check class="w-3.5 h-3.5" /> Syncing</>
                  : "Start Syncing"}
              </Button>
            }
          >
            <Button
              variant="secondary"
              size="sm"
              loading={loadingAction() === "connect"}
              class={doneAction() === "connect" ? "pointer-events-none min-w-[120px] !bg-emerald-600 !text-white !border-emerald-600" : isBusy() && loadingAction() !== "connect" ? "pointer-events-none min-w-[120px]" : "min-w-[120px]"}
              onClick={() => doAdd("connect")}
            >
              {loadingAction() === "connect"
                ? "Connecting..."
                : doneAction() === "connect"
                ? <><Check class="w-3.5 h-3.5" /> Connected</>
                : "Connect"}
            </Button>
            <Button
              variant="primary"
              size="sm"
              loading={loadingAction() === "replace"}
              class={doneAction() === "replace" ? "pointer-events-none min-w-[120px] !bg-emerald-600" : isBusy() && loadingAction() !== "replace" ? "pointer-events-none min-w-[120px]" : "min-w-[120px]"}
              onClick={() => doAdd("replace")}
            >
              {loadingAction() === "replace"
                ? "Replacing..."
                : doneAction() === "replace"
                ? <><Check class="w-3.5 h-3.5" /> Replaced</>
                : "Replace"}
            </Button>
          </Show>
        </DialogFooter>

        {/* Non-conflict errors */}
        <Show when={error()}>
          <div class="px-5 pb-4 -mt-2">
            <div class="text-sm text-red-600 bg-red-50 rounded-md px-3 py-2">
              {error()}
            </div>
          </div>
        </Show>
      </Dialog>

      {/* Open-folder error banner */}
      <Show when={openError()}>
        <div class="text-sm text-red-600 bg-red-50 border border-red-200 rounded-md px-3 py-2">
          {openError()}
        </div>
      </Show>

      {/* Folder List */}
      <Show
        when={folders() && folders()!.length > 0}
        fallback={
          <div class="flex flex-col items-center justify-center rounded-lg border border-dashed border-zinc-300 bg-zinc-50/50 py-12 px-6">
            <FolderOpen class="w-8 h-8 text-zinc-400 mb-3" />
            <p class="text-sm text-zinc-500">
              No folders added yet. Click "Add Folder" to get started.
            </p>
          </div>
        }
      >
        <div class="hidden md:block bg-white border border-zinc-200 rounded-lg shadow-sm overflow-hidden">
          {/* Table header */}
          <div class="grid grid-cols-[2fr_160px_90px_96px] items-center px-4 py-2 bg-zinc-50 border-b border-zinc-200">
            <div class="text-xs font-medium text-zinc-500 uppercase tracking-wide">Name</div>
            <div class="text-xs font-medium text-zinc-500 uppercase tracking-wide">Last Sync</div>
            <div class="text-xs font-medium text-zinc-500 uppercase tracking-wide">Status</div>
            <div />
          </div>

          {/* Table rows */}
          <div class="divide-y divide-zinc-100">
            <For each={folders()}>
              {(folder) => (
                <div class="grid grid-cols-[2fr_160px_90px_96px] items-center px-4 py-3 hover:bg-zinc-50/60 transition-colors">
                  {/* Name */}
                  <div class="flex items-center gap-1.5 min-w-0 pr-3">
                    <div class="min-w-0">
                      <div class="flex items-center gap-1 min-w-0">
                        <span class="text-sm font-medium text-zinc-900 truncate">
                          {getFolderName(folder.local_path)}
                        </span>
                        <span
                          title={folder.local_path}
                          class="shrink-0 text-zinc-400 hover:text-zinc-600 cursor-default"
                        >
                          <Info class="w-3 h-3" />
                        </span>
                      </div>
                      <Show when={folder.mode === "cloud_only"}>
                        <span class="text-[10px] text-zinc-400 font-medium uppercase tracking-wide">Cloud Only</span>
                      </Show>
                    </div>
                  </div>

                  {/* Last Sync */}
                  <div class="min-w-0 pr-3">
                    <div class="text-xs text-zinc-500 truncate">
                      {formatDateShort(folder.last_sync_at)}
                    </div>
                  </div>

                  {/* Status */}
                  <div>
                    <Show
                      when={folder.status === "syncing" && progress()[folder.id]}
                      fallback={
                        <Badge variant={statusVariant(folder.status)}>
                          {folder.status}
                        </Badge>
                      }
                    >
                      {(() => {
                        const p = () => progress()[folder.id];
                        const pct = () => typeof p()?.percent === "number" ? p()!.percent! : null;
                        const file = () => p()?.current_file;
                        const isCancelling = () => !!cancelling()[folder.id];
                        return (
                          <div class="flex items-center gap-1.5 min-w-0">
                            <div class="flex-1 min-w-0">
                              <div class="flex items-center justify-between text-[10px] text-zinc-500 mb-0.5">
                                <span class="truncate">
                                  {file() ? file()!.split("/").pop() : "syncing..."}
                                </span>
                                <span class="shrink-0 ml-1 tabular-nums">
                                  {pct() !== null ? `${pct()}%` : ""}
                                </span>
                              </div>
                              <div class="h-1 w-full bg-zinc-100 rounded-full overflow-hidden">
                                <div
                                  class="h-full bg-amber-500 transition-all"
                                  style={{
                                    width: `${pct() ?? 0}%`,
                                  }}
                                />
                              </div>
                            </div>
                            <button
                              class="shrink-0 w-5 h-5 flex items-center justify-center rounded hover:bg-zinc-200 text-zinc-500 hover:text-zinc-900 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                              title="Cancel sync"
                              onClick={() => handleCancel(folder.id)}
                              disabled={isCancelling()}
                            >
                              <Show
                                when={isCancelling()}
                                fallback={<X class="w-3 h-3" />}
                              >
                                <Loader2 class="w-3 h-3 animate-spin" />
                              </Show>
                            </button>
                          </div>
                        );
                      })()}
                    </Show>
                  </div>

                  {/* Actions */}
                  <div class="flex items-center gap-1 justify-end">
                    <IconButton
                      icon={<ExternalLink class="w-3.5 h-3.5" />}
                      title="Open folder"
                      onClick={() => handleOpenFolder(folder.local_path)}
                    />
                    <Show when={folder.mode !== "cloud_only"}>
                      <IconButton
                        icon={<CloudOff class="w-3.5 h-3.5" />}
                        title="Release local files"
                        loading={syncingId() === `release:${folder.id}`}
                        onClick={() => handleRelease(folder.id)}
                        disabled={syncingId() !== null}
                      />
                    </Show>
                    <IconButton
                      icon={<Trash2 class="w-3.5 h-3.5" />}
                      title="Remove folder"
                      loading={syncingId() === `delete:${folder.id}`}
                      onClick={() => handleDelete(folder.id)}
                      danger
                      disabled={syncingId() !== null}
                    />
                  </div>
                </div>
              )}
            </For>
          </div>
        </div>

        {/* Mobile card list */}
        <div class="md:hidden space-y-3">
          <For each={folders()}>
            {(folder) => (
              <div class="bg-white border border-zinc-200 rounded-lg p-4 space-y-3">
                {/* Name + status */}
                <div class="flex items-center justify-between gap-2">
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-1.5">
                      <span class="text-sm font-medium text-zinc-900 truncate">
                        {getFolderName(folder.local_path)}
                      </span>
                      <Show when={folder.mode === "cloud_only"}>
                        <span class="text-[10px] text-zinc-400 font-medium uppercase tracking-wide shrink-0">Cloud Only</span>
                      </Show>
                    </div>
                    <div class="text-xs text-zinc-400 truncate mt-0.5">
                      {getFolderDir(folder.local_path)}
                    </div>
                  </div>
                  <Show
                    when={folder.status === "syncing" && progress()[folder.id]}
                    fallback={
                      <Badge variant={statusVariant(folder.status)}>
                        {folder.status}
                      </Badge>
                    }
                  >
                    {(() => {
                      const p = () => progress()[folder.id];
                      const pct = () => typeof p()?.percent === "number" ? p()!.percent! : null;
                      const isCancelling = () => !!cancelling()[folder.id];
                      return (
                        <div class="flex items-center gap-1.5 shrink-0 w-28">
                          <div class="flex-1 min-w-0">
                            <div class="text-[10px] text-zinc-500 mb-0.5 truncate text-right">
                              {pct() !== null ? `${pct()}%` : "..."}
                            </div>
                            <div class="h-1 w-full bg-zinc-100 rounded-full overflow-hidden">
                              <div
                                class="h-full bg-amber-500 transition-all"
                                style={{ width: `${pct() ?? 0}%` }}
                              />
                            </div>
                          </div>
                          <button
                            class="shrink-0 w-5 h-5 flex items-center justify-center rounded hover:bg-zinc-200 text-zinc-500 hover:text-zinc-900 transition-colors disabled:opacity-50"
                            title="Cancel sync"
                            onClick={() => handleCancel(folder.id)}
                            disabled={isCancelling()}
                          >
                            <Show
                              when={isCancelling()}
                              fallback={<X class="w-3 h-3" />}
                            >
                              <Loader2 class="w-3 h-3 animate-spin" />
                            </Show>
                          </button>
                        </div>
                      );
                    })()}
                  </Show>
                </div>
                {/* Last sync + actions */}
                <div class="flex items-center justify-between">
                  <span class="text-xs text-zinc-500">
                    {formatDateShort(folder.last_sync_at)}
                  </span>
                  <div class="flex items-center gap-1">
                    <IconButton
                      icon={<ExternalLink class="w-3.5 h-3.5" />}
                      title="Open folder"
                      onClick={() => handleOpenFolder(folder.local_path)}
                    />
                    <Show when={folder.mode !== "cloud_only"}>
                      <IconButton
                        icon={<CloudOff class="w-3.5 h-3.5" />}
                        title="Release local files"
                        loading={syncingId() === `release:${folder.id}`}
                        onClick={() => handleRelease(folder.id)}
                        disabled={syncingId() !== null}
                      />
                    </Show>
                    <IconButton
                      icon={<Trash2 class="w-3.5 h-3.5" />}
                      title="Remove folder"
                      loading={syncingId() === `delete:${folder.id}`}
                      onClick={() => handleDelete(folder.id)}
                      danger
                      disabled={syncingId() !== null}
                    />
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
