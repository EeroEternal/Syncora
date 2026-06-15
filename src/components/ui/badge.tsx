import { type ParentProps, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

interface BadgeProps extends ParentProps {
  variant?: "default" | "success" | "warning" | "error" | "outline";
  class?: string;
}

const variants: Record<string, string> = {
  default: "bg-zinc-100 text-zinc-700",
  success: "bg-emerald-50 text-emerald-700",
  warning: "bg-amber-50 text-amber-700",
  error: "bg-red-50 text-red-700",
  outline: "border border-zinc-300 text-zinc-600",
};

export function Badge(props: BadgeProps) {
  const [local, rest] = splitProps(props, ["variant", "class", "children"]);

  return (
    <span
      class={cn(
        "inline-flex items-center gap-1 min-w-[5rem] h-5 text-xs font-medium rounded-full px-2 justify-center",
        variants[local.variant || "default"],
        local.class
      )}
      {...rest}
    >
      {local.children}
    </span>
  );
}
