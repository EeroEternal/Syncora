import { type JSX, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

interface InputProps extends JSX.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

/** Console input — aligned with ParaRouter consoleInputClass */
export function Input(props: InputProps) {
  const [local, rest] = splitProps(props, ["class", "label"]);
  return (
    <div class="space-y-2">
      {local.label && (
        <label class="block text-xs font-semibold uppercase tracking-wider text-zinc-500">
          {local.label}
        </label>
      )}
      <input
        class={cn(
          "w-full bg-white border border-zinc-300 rounded-md px-3 py-2 text-sm text-zinc-900 placeholder:text-zinc-400 focus:outline-none focus:border-black focus:ring-1 focus:ring-black transition-colors disabled:opacity-50 disabled:pointer-events-none",
          local.class
        )}
        {...rest}
      />
    </div>
  );
}

/** Console form label — aligned with ParaRouter consoleFormLabelClass */
export function FormLabel(props: { children: any; class?: string }) {
  return (
    <label
      class={cn(
        "block text-xs font-semibold uppercase tracking-wider text-zinc-500",
        props.class
      )}
    >
      {props.children}
    </label>
  );
}
