import { type JSX, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

interface InputProps extends JSX.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export function Input(props: InputProps) {
  const [local, rest] = splitProps(props, ["class", "label"]);
  return (
    <div class="space-y-2">
      {local.label && (
        <label class="text-sm font-medium leading-none">{local.label}</label>
      )}
      <input
        class={cn(
          "flex h-9 w-full rounded-md border border-[hsl(var(--input))] bg-transparent px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-[hsl(var(--muted-foreground))] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[hsl(var(--ring))] disabled:cursor-not-allowed disabled:opacity-50",
          local.class
        )}
        {...rest}
      />
    </div>
  );
}
