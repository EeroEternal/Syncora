import { createSignal } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { login, register } from "~/lib/tauri";

export default function Login() {
  const navigate = useNavigate();
  const [isRegister, setIsRegister] = createSignal(false);
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [error, setError] = createSignal("");
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async () => {
    if (!email() || !password()) {
      setError("Please fill in all fields");
      return;
    }
    if (password().length < 8) {
      setError("Password must be at least 8 characters");
      return;
    }

    setLoading(true);
    setError("");

    try {
      if (isRegister()) {
        await register(email(), password());
      } else {
        await login(email(), password());
      }
      navigate("/");
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="flex min-h-screen items-center justify-center bg-zinc-50">
      <div class="w-full max-w-sm bg-white border border-zinc-200 shadow-sm rounded-lg p-6 space-y-6">
        {/* Brand */}
        <div class="text-center space-y-1">
          <h1 class="text-2xl font-bold tracking-tight text-zinc-900">
            Syncora
          </h1>
          <p class="text-sm text-zinc-500">
            {isRegister()
              ? "Create an account to start syncing"
              : "Sign in to your account"}
          </p>
        </div>

        {/* Error */}
        {error() && (
          <div class="text-sm text-red-600 bg-red-50 rounded-md px-3 py-2 border border-red-200">
            {error()}
          </div>
        )}

        {/* Form */}
        <div class="space-y-4">
          <Input
            label="Email"
            type="email"
            placeholder="you@example.com"
            value={email()}
            onInput={(e) => setEmail(e.currentTarget.value)}
          />
          <Input
            label="Password"
            type="password"
            placeholder="At least 8 characters"
            value={password()}
            onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
            onInput={(e) => setPassword(e.currentTarget.value)}
          />

          <Button
            class="w-full"
            variant="primary"
            onClick={handleSubmit}
            loading={loading()}
          >
            {loading()
              ? "Please wait..."
              : isRegister()
              ? "Create Account"
              : "Sign In"}
          </Button>
        </div>

        {/* Toggle */}
        <div class="text-center text-sm text-zinc-500">
          {isRegister() ? "Already have an account?" : "Don't have an account?"}{" "}
          <button
            class="text-zinc-900 font-medium hover:underline"
            onClick={() => {
              setIsRegister(!isRegister());
              setError("");
            }}
          >
            {isRegister() ? "Sign In" : "Sign Up"}
          </button>
        </div>
      </div>
    </div>
  );
}
