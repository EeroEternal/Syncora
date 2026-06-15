import { type JSX, type ParentProps, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

interface CardProps extends ParentProps {
  class?: string;
}

export function Card(props: CardProps) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <div
      class={cn(
        "bg-white border border-zinc-200 rounded-lg shadow-sm",
        local.class
      )}
      {...rest}
    >
      {local.children}
    </div>
  );
}

export function CardHeader(props: CardProps) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <div class={cn("flex flex-col space-y-1.5 p-6", local.class)} {...rest}>
      {local.children}
    </div>
  );
}

export function CardTitle(props: CardProps) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <h3
      class={cn("text-lg font-semibold leading-none tracking-tight text-zinc-900", local.class)}
      {...rest}
    >
      {local.children}
    </h3>
  );
}

export function CardDescription(props: CardProps) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <p class={cn("text-sm text-zinc-500", local.class)} {...rest}>
      {local.children}
    </p>
  );
}

export function CardContent(props: CardProps) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <div class={cn("p-6 pt-0", local.class)} {...rest}>
      {local.children}
    </div>
  );
}
