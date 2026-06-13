import { createResource, createSignal } from "solid-js";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Badge } from "~/components/ui/badge";
import { getSettings, saveSettings, testR2Connection } from "~/lib/tauri";
import type { Settings as SettingsType } from "~/lib/tauri";

export default function Settings() {
  const [settings, { refetch }] = createResource(getSettings);
  const [saving, setSaving] = createSignal(false);
  const [testing, setTesting] = createSignal(false);
  const [testResult, setTestResult] = createSignal<{ success: boolean; message: string } | null>(null);

  // Local form state
  const [endpoint, setEndpoint] = createSignal("");
  const [accessKey, setAccessKey] = createSignal("");
  const [secret, setSecret] = createSignal("");
  const [bucket, setBucket] = createSignal("");
  const [interval, setInterval] = createSignal(5);

  // Initialize form when settings load
  const initForm = () => {
    const s = settings();
    if (s) {
      setEndpoint(s.r2_endpoint || "");
      setAccessKey(s.r2_access_key || "");
      setSecret(s.r2_secret || "");
      setBucket(s.r2_bucket || "");
      setInterval(s.sync_interval_minutes || 5);
    }
  };

  // Watch for settings load
  createResource(() => settings(), initForm);

  const handleSave = async () => {
    setSaving(true);
    try {
      await saveSettings({
        r2_endpoint: endpoint(),
        r2_access_key: accessKey(),
        r2_secret: secret(),
        r2_bucket: bucket(),
        sync_interval_minutes: interval(),
        auto_start: false,
      });
      refetch();
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const result = await testR2Connection();
      setTestResult(result);
    } catch (e: any) {
      setTestResult({ success: false, message: e.toString() });
    } finally {
      setTesting(false);
    }
  };

  return (
    <div class="space-y-6">
      <div>
        <h1 class="text-2xl font-bold">Settings</h1>
        <p class="text-sm text-[hsl(var(--muted-foreground))]">
          Configure your Cloudflare R2 connection and sync preferences
        </p>
      </div>

      {/* R2 Configuration */}
      <Card>
        <CardHeader>
          <CardTitle>Cloudflare R2 Connection</CardTitle>
          <CardDescription>
            Enter your R2 credentials to enable file synchronization
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <Input
            label="R2 Endpoint"
            placeholder="https://xxx.r2.cloudflarestorage.com"
            value={endpoint()}
            onInput={(e) => setEndpoint(e.currentTarget.value)}
          />
          <Input
            label="Bucket Name"
            placeholder="my-sync-bucket"
            value={bucket()}
            onInput={(e) => setBucket(e.currentTarget.value)}
          />
          <Input
            label="Access Key ID"
            placeholder="Enter your access key"
            value={accessKey()}
            onInput={(e) => setAccessKey(e.currentTarget.value)}
          />
          <Input
            label="Secret Access Key"
            type="password"
            placeholder="Enter your secret key"
            value={secret()}
            onInput={(e) => setSecret(e.currentTarget.value)}
          />

          <div class="flex items-center gap-3 pt-2">
            <Button variant="outline" onClick={handleTest} disabled={testing()}>
              {testing() ? "Testing..." : "Test Connection"}
            </Button>
            {testResult() && (
              <Badge variant={testResult()!.success ? "success" : "destructive"}>
                {testResult()!.message}
              </Badge>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Sync Settings */}
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

      {/* Save */}
      <div class="flex justify-end">
        <Button onClick={handleSave} disabled={saving()}>
          {saving() ? "Saving..." : "Save Settings"}
        </Button>
      </div>
    </div>
  );
}
