import { type ParentProps } from "solid-js";
import Sidebar from "./components/layout/sidebar";

export default function App(props: ParentProps) {
  return (
    <div class="flex h-screen w-screen">
      <Sidebar />
      <main class="flex-1 overflow-auto bg-[hsl(var(--background))] p-6">
        {props.children}
      </main>
    </div>
  );
}
