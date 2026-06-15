import { type JSX, Show, splitProps, onMount, onCleanup } from "solid-js";
import { Portal } from "solid-js/web";
import { cn } from "~/lib/utils";

type DialogSize = "sm" | "md" | "lg";

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  subtitle?: string;
  children: JSX.Element;
  size?: DialogSize;
  /** Structured dialog with Header / Body / Footer sections */
  structured?: boolean;
  class?: string;
}

const sizeClasses: Record<DialogSize, string> = {
  sm: "w-full max-w-sm max-w-[calc(100vw-2rem)]",
  md: "w-[480px] max-w-[calc(100vw-2rem)]",
  lg: "w-[560px] max-w-[calc(100vw-2rem)]",
};

const panelBase =
  "relative bg-white border border-zinc-200 shadow-sm rounded-lg text-left flex flex-col overflow-hidden pointer-events-auto";

export function Dialog(props: DialogProps) {
  const size = () => props.size || "md";

  // Escape key closes dialog
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape" && props.open) {
      e.preventDefault();
      props.onClose();
    }
  };

  onMount(() => document.addEventListener("keydown", onKeyDown));
  onCleanup(() => document.removeEventListener("keydown", onKeyDown));

  return (
    <Show when={props.open}>
      <Portal>
        {/* Backdrop */}
        <div
          class="fixed inset-0 z-[100] bg-black/50 transition-opacity"
          aria-hidden
          onClick={props.onClose}
        />
        {/* Shell */}
        <div class="fixed inset-0 z-[101] flex items-center justify-center p-4 pointer-events-none">
          <div
            class={cn(
              panelBase,
              props.structured
                ? cn(sizeClasses[size()], "max-h-[90vh] p-0")
                : sizeClasses[size()],
              props.class
            )}
            role="dialog"
            aria-modal="true"
          >
            {/* Structured header */}
            <Show when={props.structured && (props.title || props.subtitle)}>
              <div class="px-5 py-4 border-b border-zinc-200 shrink-0 bg-white">
                <h3 class="font-bold text-lg text-zinc-900">{props.title}</h3>
                <Show when={props.subtitle}>
                  <p class="text-xs text-zinc-500 mt-0.5">{props.subtitle}</p>
                </Show>
              </div>
            </Show>

            {/* Simple dialog: inline title + content */}
            <Show when={!props.structured}>
              <div class="p-6 space-y-4">
                <Show when={props.title}>
                  <h3 class="font-bold text-lg text-zinc-900">{props.title}</h3>
                </Show>
                <Show when={props.subtitle}>
                  <p class="text-xs text-zinc-500">{props.subtitle}</p>
                </Show>
                {props.children}
              </div>
            </Show>

            {/* Structured: children rendered as-is (use DialogBody / DialogFooter) */}
            <Show when={props.structured}>{props.children}</Show>
          </div>
        </div>
      </Portal>
    </Show>
  );
}

/** Structured dialog Body section — scrollable, px-5 py-5 */
export function DialogBody(props: { children: JSX.Element; class?: string }) {
  return (
    <div
      class={cn(
        "flex-1 min-h-0 overflow-y-auto px-5 py-5 space-y-4",
        props.class
      )}
    >
      {props.children}
    </div>
  );
}

/** Structured dialog Footer section — border-t, bg-zinc-50 */
export function DialogFooter(props: { children: JSX.Element; class?: string }) {
  return (
    <div
      class={cn(
        "border-t border-zinc-200 px-6 py-4 bg-zinc-50/80 flex justify-end gap-2 shrink-0",
        props.class
      )}
    >
      {props.children}
    </div>
  );
}
