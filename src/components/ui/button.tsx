import { type JSX, splitProps, Show } from "solid-js";
import { cn } from "~/lib/utils";

/** Pure CSS spinner — border ring, inherits currentColor */
export function Spinner(props: { size?: "sm" | "md" | "lg"; class?: string }) {
  const s = props.size || "sm";
  return (
    <span
      class={cn("spinner", s === "sm" ? "spinner-sm" : s === "md" ? "spinner-md" : "spinner-lg", props.class)}
      role="status"
      aria-label="Loading"
    />
  );
}

interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
  loading?: boolean;
}

const buttonBase =
  "inline-flex items-center justify-center gap-2 font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2";

const buttonSizes: Record<string, string> = {
  sm: "h-9 px-3 text-sm rounded-md min-w-[88px]",
  md: "h-9 px-4 text-sm rounded-md min-w-[88px]",
  lg: "px-8 py-3 text-base rounded-md min-w-[120px]",
};

const buttonVariants: Record<string, string> = {
  primary: "bg-black text-white hover:bg-zinc-800 focus:ring-black",
  secondary:
    "bg-transparent border border-zinc-300 text-zinc-900 hover:bg-zinc-50 focus:ring-black",
  ghost:
    "p-2 min-w-0 text-zinc-500 hover:text-zinc-900 hover:bg-zinc-100 rounded-md focus:ring-black",
  danger: "bg-red-600 text-white hover:bg-red-700 focus:ring-red-500",
};

/* Loading-specific overrides — dramatic visual change */
const loadingStyles: Record<string, string> = {
  primary: "bg-zinc-500 text-white",
  secondary: "bg-zinc-100 border-zinc-300 text-zinc-500",
  ghost: "bg-zinc-100 text-zinc-400",
  danger: "bg-zinc-400 text-white",
};

export function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, [
    "variant",
    "size",
    "loading",
    "class",
    "children",
    "disabled",
  ]);

  const variant = () => local.variant || "primary";
  const size = () => local.size || "md";
  const isLoading = () => local.loading === true;

  return (
    <button
      class={cn(
        buttonBase,
        buttonSizes[size()],
        variant() === "ghost" ? buttonVariants.ghost : buttonVariants[variant()],
        // Loading: swap to loading style + pulse.
        // Use `cursor-progress` (not `cursor-wait`): on macOS, `cursor-wait` renders
        // the rainbow beach-ball, which makes the app look frozen even though the
        // backend call is just running normally. `cursor-progress` shows a small
        // spinner next to the arrow — "working in background, UI still responsive".
        isLoading() && cn(loadingStyles[variant()] || loadingStyles.primary, "cursor-progress"),
        // Disabled (not loading): standard dim
        local.disabled && !isLoading() && "opacity-50 cursor-not-allowed",
        local.class
      )}
      disabled={local.disabled || isLoading()}
      aria-busy={isLoading()}
      {...rest}
    >
      <Show when={isLoading()} fallback={local.children}>
        <Spinner size={size() === "lg" ? "md" : "sm"} />
        {local.children}
      </Show>
    </button>
  );
}

/** Icon-only button for row actions / toolbar */
export function IconButton(props: {
  icon: JSX.Element;
  title: string;
  onClick?: () => void;
  disabled?: boolean;
  danger?: boolean;
  class?: string;
  loading?: boolean;
}) {
  const base = props.danger
    ? "inline-flex items-center justify-center rounded-md p-1.5 text-red-500 hover:bg-red-50 hover:text-red-700 disabled:opacity-40 disabled:pointer-events-none focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-1 transition-all duration-200"
    : "inline-flex items-center justify-center rounded-md p-1.5 text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 disabled:opacity-40 disabled:pointer-events-none focus:outline-none focus:ring-2 focus:ring-black focus:ring-offset-1 transition-all duration-200";

  return (
    <button
      class={cn(
        base,
        // See note in `Button` above: avoid `cursor-wait` on macOS (beach-ball cursor).
        props.loading && "cursor-progress bg-zinc-100",
        props.class
      )}
      title={props.title}
      aria-label={props.title}
      onClick={props.onClick}
      disabled={props.disabled || props.loading}
    >
      <Show when={props.loading} fallback={props.icon}>
        <Spinner size="sm" />
      </Show>
    </button>
  );
}
