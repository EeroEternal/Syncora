import { type ParentProps, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

interface BadgeProps extends ParentProps {
  variant?: "default" | "success" | "warning" | "destructive" | "outline" | "conflict";
  class?: string;
}

export function Badge(props: BadgeProps) {
  const [local, rest] = splitProps(props, ["variant", "class", "children"]);

  const variants: Record<string, string> = {
    default: "bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]",
    success: "bg-[hsl(var(--success))] text-white",
    warning: "bg-[hsl(var(--warning))] text-white",
    destructive: "bg-[hsl(var(--destructive))] text-[hsl(var(--destructive-foreground))]",
    outline: "border border-[hsl(var(--border))] text-[hsl(var(--foreground))]",
    conflict: "bg-[hsl(var(--conflict))] text-white",
  };

  return (
    <span
      class={cn(
        "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold transition-colors",
        variants[local.variant || "default"],
        local.class
      )}
      {...rest}
    >
      {local.children}
    </span>
  );
}
