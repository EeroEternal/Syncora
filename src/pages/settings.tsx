import { createResource, createSignal } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { getSettings, saveSettings, getAuthStatus, logout } from "~/lib/tauri";
import type { Settings as SettingsType, AuthStatus } from "~/lib/tauri";
import { LogOut, Save } from "lucide-solid";

export default function Settings() {
  const navigate = useNavigate();
  const [settings, { refetch }] = createResource(getSettings);
  const [authStatus] = createResource<AuthStatus>(getAuthStatus);
  const [saving, setSaving] = createSignal(false);

  const [apiUrl, setApiUrl] = createSignal("");
  const [interval, setInterval] = createSignal(5);
  const [autoStart, setAutoStart] = createSignal(false);

  const initForm = () => {
    const s = settings();
    if (s) {
      setApiUrl(s.api_base_url || "https://api.synchora.cc");
      setInterval(s.sync_interval_minutes || 5);
      setAutoStart(s.auto_start ?? false);
    }
  };

  createResource(() => settings(), initForm);

  const handleSave = async () => {
    setSaving(true);
    try {
      await saveSettings({
        api_base_url: apiUrl(),
        sync_interval_minutes: interval(),
        auto_start: autoStart(),
      });
      refetch();
    } finally {
      setSaving(false);
    }
  };

  const handleLogout = async () => {
    try {
      await logout();
      navigate("/login");
    } catch (e) {
      console.error("Logout failed:", e);
    }
  };

  return (
    <div class="space-y-6">
      <div>
        <h1 class="text-2xl font-bold tracking-tight text-zinc-900">Settings</h1>
        <p class="text-sm text-zinc-500">
          Manage your account and sync preferences
        </p>
      </div>

      {/* Account */}
      <Card>
        <CardHeader>
          <CardTitle>Account</CardTitle>
          <CardDescription>Your Syncora account information</CardDescription>
        </CardHeader>
        <CardContent>
          {authStatus()?.logged_in ? (
            <div class="space-y-3">
              <div class="flex items-center gap-2">
                <span class="text-xs font-semibold uppercase tracking-wider text-zinc-500">
                  Email
                </span>
                <span class="text-sm text-zinc-900">
                  {authStatus()?.user?.email}
                </span>
              </div>
              {authStatus()?.user?.display_name && (
                <div class="flex items-center gap-2">
                  <span class="text-xs font-semibold uppercase tracking-wider text-zinc-500">
                    Name
                  </span>
                  <span class="text-sm text-zinc-900">
                    {authStatus()?.user?.display_name}
                  </span>
                </div>
              )}
              <div class="pt-2">
                <Button variant="secondary" size="sm" onClick={handleLogout}>
                  <LogOut class="w-3.5 h-3.5" />
                  Sign Out
                </Button>
              </div>
            </div>
          ) : (
            <div class="space-y-3">
              <p class="text-sm text-zinc-500">Not signed in</p>
              <Button variant="primary" size="sm" onClick={() => navigate("/login")}>
                Sign In
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Sync Preferences */}
      <Card>
        <CardHeader>
          <CardTitle>Sync Preferences</CardTitle>
        </CardHeader>
        <CardContent class="space-y-4">
          <Input
            label="Sync Interval (minutes)"
            type="number"
            min="1"
            max="1440"
            value={interval()}
            onInput={(e) => setInterval(parseInt(e.currentTarget.value) || 5)}
          />
        </CardContent>
      </Card>

      {/* Application */}
      <Card>
        <CardHeader>
          <CardTitle>Application</CardTitle>
          <CardDescription>App behavior preferences</CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="flex items-center justify-between">
            <div class="space-y-0.5">
              <p class="text-sm font-medium text-zinc-900">Launch at Login</p>
              <p class="text-xs text-zinc-500">Start Syncora automatically when you log in</p>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={autoStart()}
              onClick={() => setAutoStart(!autoStart())}
              class={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${autoStart() ? 'bg-zinc-900' : 'bg-zinc-200'}`}
            >
              <span
                class={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition-transform duration-200 ease-in-out ${autoStart() ? 'translate-x-4' : 'translate-x-0'}`}
              />
            </button>
          </div>
          <div class="flex items-center justify-between">
            <div class="space-y-0.5">
              <p class="text-sm font-medium text-zinc-900">Close to Tray</p>
              <p class="text-xs text-zinc-500">Keep running in the background when the window is closed</p>
            </div>
            <span class="inline-flex items-center rounded-md bg-zinc-100 px-2 py-0.5 text-xs font-medium text-zinc-600">Always on</span>
          </div>
        </CardContent>
      </Card>

      {/* Advanced */}
      <Card>
        <CardHeader>
          <CardTitle>Advanced</CardTitle>
          <CardDescription>API server configuration</CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <Input
            label="API Base URL"
            placeholder="https://api.synchora.cc"
            value={apiUrl()}
            class="font-mono"
            onInput={(e) => setApiUrl(e.currentTarget.value)}
          />
        </CardContent>
      </Card>

      {/* Save */}
      <div class="flex justify-end">
        <Button variant="primary" size="sm" onClick={handleSave} loading={saving()} class="min-w-[140px]">
          <Save class="w-3.5 h-3.5" />
          {saving() ? "Saving..." : "Save Settings"}
        </Button>
      </div>
    </div>
  );
}

