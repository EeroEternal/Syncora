import { type JSX, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "destructive" | "outline" | "secondary" | "ghost";
  size?: "default" | "sm" | "lg" | "icon";
}

export function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, ["variant", "size", "class", "children"]);

  const baseClass = "inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--ring))] disabled:pointer-events-none disabled:opacity-50";

  const variants: Record<string, string> = {
    default: "bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))] hover:opacity-90",
    destructive: "bg-[hsl(var(--destructive))] text-[hsl(var(--destructive-foreground))] hover:opacity-90",
    outline: "border border-[hsl(var(--input))] bg-transparent hover:bg-[hsl(var(--accent))] hover:text-[hsl(var(--accent-foreground))]",
    secondary: "bg-[hsl(var(--secondary))] text-[hsl(var(--secondary-foreground))] hover:opacity-80",
    ghost: "hover:bg-[hsl(var(--accent))] hover:text-[hsl(var(--accent-foreground))]",
  };

  const sizes: Record<string, string> = {
    default: "h-9 px-4 py-2",
    sm: "h-8 rounded-md px-3 text-xs",
    lg: "h-10 rounded-md px-8",
    icon: "h-9 w-9",
  };

  return (
    <button
      class={cn(
        baseClass,
        variants[local.variant || "default"],
        sizes[local.size || "default"],
        local.class
      )}
      {...rest}
    >
      {local.children}
    </button>
  );
}
