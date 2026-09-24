import { type FormEvent, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { APP_NAME } from "@/lib/t-books/constants";
import { useBooks } from "@/lib/t-books/store";
import { ErrorBanner } from "./error-banner";
import { Monogram } from "./monogram";
import { StatusBanners } from "./status-banners";

type Mode = "login" | "setup";

export function SignInScreen() {
  const inspect = useBooks((s) => s.inspect);
  const login = useBooks((s) => s.login);
  const setPassword = useBooks((s) => s.setPassword);
  const ensureReady = useBooks((s) => s.ensureReady);
  const boot = useBooks((s) => s.boot);

  const [email, setEmail] = useState("");
  const [password, setPasswordValue] = useState("");
  const [confirm, setConfirm] = useState("");
  const [mode, setMode] = useState<Mode>("login");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (boot === "ready" && email) void applyModeFromEmail(email);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- only when books become ready
  }, [boot]);

  async function applyModeFromEmail(value: string) {
    if (boot !== "ready") return;
    try {
      const next = await inspect(value);
      setMode(next.kind === "set-password" ? "setup" : "login");
    } catch {
      setMode("login");
    }
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await ensureReady();
      const next = await inspect(email);
      if (next.kind === "denied") {
        const result = await login(email, password);
        if (!result.ok) {
          const again = await inspect(email);
          if (again.kind === "set-password") {
            setMode("setup");
            setError(null);
            return;
          }
          if (again.kind === "denied") {
            setError(again.message);
            setMode("login");
            return;
          }
          setError(result.message);
        }
        return;
      }
      if (next.kind === "set-password" && mode !== "setup") {
        setMode("setup");
        return;
      }
      if (next.kind === "set-password") {
        const result = await setPassword(email, password, confirm);
        if (!result.ok) setError(result.message);
        return;
      }
      const result = await login(email, password);
      if (!result.ok) {
        if (result.message.includes("Set a password")) {
          setMode("setup");
          setError(null);
          return;
        }
        setError(result.message);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Sign-in failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex min-h-dvh flex-col bg-paper text-ink">
      <StatusBanners />
      <main className="mx-auto flex w-full max-w-md flex-1 flex-col justify-center px-5 py-10">
        <header className="enter mb-8 flex items-center gap-3">
          <Monogram />
          <div>
            <p className="text-2xl font-medium leading-tight tracking-tight">{APP_NAME}</p>
            <p className="text-sm text-ink-muted">Office register</p>
          </div>
        </header>

        <form
          onSubmit={onSubmit}
          className="enter enter-delay-1 rounded-lg bg-paper-raised p-6 ring-1 ring-line"
        >
          <h1 className="text-xl font-medium tracking-tight">
            {mode === "setup" ? "Set a password" : "Sign in"}
          </h1>

          <div className="mt-5 space-y-4">
            {error ? <ErrorBanner message={error} /> : null}

            <div className="space-y-1.5">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                name="email"
                type="email"
                autoComplete="username"
                autoFocus
                value={email}
                onChange={(e) => {
                  const value = e.target.value;
                  setEmail(value);
                  setError(null);
                  void applyModeFromEmail(value);
                }}
                disabled={busy}
                required
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="password">{mode === "setup" ? "New password" : "Password"}</Label>
              <Input
                id="password"
                name="password"
                type="password"
                autoComplete={mode === "setup" ? "new-password" : "current-password"}
                value={password}
                onChange={(e) => setPasswordValue(e.target.value)}
                disabled={busy}
                minLength={8}
                required
              />
            </div>

            {mode === "setup" ? (
              <div className="space-y-1.5">
                <Label htmlFor="confirm">Confirm password</Label>
                <Input
                  id="confirm"
                  name="confirm"
                  type="password"
                  autoComplete="new-password"
                  value={confirm}
                  onChange={(e) => setConfirm(e.target.value)}
                  disabled={busy}
                  minLength={8}
                  required
                />
              </div>
            ) : null}
          </div>

          <div className="mt-6">
            <Button type="submit" disabled={busy} className="w-full">
              {busy ? "Working\u2026" : "Sign In"}
            </Button>
          </div>
        </form>
      </main>
    </div>
  );
}
