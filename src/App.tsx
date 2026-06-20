import { type ParentProps, onMount, onCleanup } from "solid-js";
import { useNavigate } from "@solidjs/router";
import Sidebar from "./components/layout/sidebar";
import { AUTH_EXPIRED_EVENT } from "~/lib/tauri";

export default function App(props: ParentProps) {
  const navigate = useNavigate();

  onMount(() => {
    const handler = () => navigate("/login");
    window.addEventListener(AUTH_EXPIRED_EVENT, handler);
    onCleanup(() => window.removeEventListener(AUTH_EXPIRED_EVENT, handler));
  });

  return (
    <div class="flex h-screen w-screen">
      <Sidebar />
      <main class="flex-1 overflow-auto scrollbar-hidden bg-zinc-50 p-8">
        {props.children}
      </main>
    </div>
  );
}
