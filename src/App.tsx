import { type ParentProps, onMount, onCleanup, createResource, createEffect } from "solid-js";
import { useNavigate } from "@solidjs/router";
import Sidebar from "./components/layout/sidebar";
import BottomNav from "./components/layout/bottom-nav";
import { AUTH_EXPIRED_EVENT, getAuthStatus } from "~/lib/tauri";

export default function App(props: ParentProps) {
  const navigate = useNavigate();
  const [authStatus] = createResource(getAuthStatus);

  // Redirect to login if session is expired / not logged in.
  // This check runs on all platforms (desktop + mobile).
  createEffect(() => {
    const status = authStatus();
    if (status && !status.logged_in) {
      navigate("/login");
    }
  });

  onMount(() => {
    const handler = () => navigate("/login");
    window.addEventListener(AUTH_EXPIRED_EVENT, handler);
    onCleanup(() => window.removeEventListener(AUTH_EXPIRED_EVENT, handler));
  });

  return (
    <div class="flex flex-col md:flex-row h-screen w-screen">
      <Sidebar />
      <main class="flex-1 overflow-auto scrollbar-hidden bg-zinc-50 p-4 md:p-8 pb-20 md:pb-8">
        {props.children}
      </main>
      <BottomNav />
    </div>
  );
}
