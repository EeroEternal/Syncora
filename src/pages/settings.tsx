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

  const initForm = () => {
    const s = settings();
    if (s) {
      setApiUrl(s.api_base_url || "https://api.synchora.cc");
      setInterval(s.sync_interval_minutes || 5);
    }
  };

  createResource(() => settings(), initForm);

  const handleSave = async () => {
    setSaving(true);
    try {
      await saveSettings({
        api_base_url: apiUrl(),
        sync_interval_minutes: interval(),
        auto_start: false,
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

