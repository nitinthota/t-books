import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { bootstrapHiveTab, hiveStatus, type HiveStatus } from "@/lib/t-books/control";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { canOpenAccess } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import {
  backupData,
  clearLogs,
  exportLogs,
  getOpsInfo,
  pickRestoreZip,
  restoreData,
  type OpsInfo,
} from "@/lib/t-books/ops";
import { ModuleFrame } from "./module-frame";
import { RefreshToast } from "./refresh-toast";

export default function SettingsScreen({ onBack }: { onBack: () => void }) {
  const [info, setInfo] = useState<OpsInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState<"restore" | "logs" | null>(null);
  const [restorePath, setRestorePath] = useState("");
  const [restoreFile, setRestoreFile] = useState<File | null>(null);
  const [hive, setHive] = useState<HiveStatus | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const session = useBooks((s) => s.session);
  const owner = session ? canOpenAccess(session.role) : false;
  const [hiveBusy, setHiveBusy] = useState(false);

  const loadLocal = useCallback(async () => {
    try {
      setInfo(await getOpsInfo());
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    }
  }, []);

  const loadHive = useCallback(async () => {
    setHiveBusy(true);
    try {
      const status = await Promise.race([
        hiveStatus(),
        new Promise<never>((_, reject) => {
          window.setTimeout(() => reject(new Error("Company book check timed out. Try again.")), 8000);
        }),
      ]);
      setHive(status);
    } catch (err) {
      setHive(null);
      setError(invokeErrorMessage(err));
    } finally {
      setHiveBusy(false);
    }
  }, []);

  useEffect(() => {
    void loadLocal();
  }, [loadLocal]);

  async function run(work: () => Promise<string | void>, ok: string) {
    setBusy(true);
    setError(null);
    try {
      const result = await work();
      setToast(typeof result === "string" && result ? result : ok);
      await loadLocal();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function chooseRestore() {
    setBusy(true);
    setError(null);
    try {
      const picked = await pickRestoreZip();
      if (picked) {
        setRestorePath(picked);
        setConfirm("restore");
      } else {
        fileRef.current?.click();
      }
    } catch {
      fileRef.current?.click();
    } finally {
      setBusy(false);
    }
  }

  return (
    <ModuleFrame title="System" onBack={onBack} error={error}>
      <section className="mb-8 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
        <h2 className="text-lg font-medium">Signed-in operator</h2>
        <dl className="mt-3 grid gap-2 text-sm sm:grid-cols-2">
          <Meta label="Email" value={session?.email ?? "\u2014"} />
          <Meta label="Role" value={session?.role ?? "\u2014"} />
          <Meta label="Version" value={info?.version ?? "\u2026"} />
          <Meta label="Data folder" value={info?.dataDir ?? "\u2026"} />
        </dl>
      </section>
      <section className="mb-8 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
        <h2 className="text-lg font-medium">Company book tabs</h2>
        <Button className="mt-3" variant="secondary" disabled={busy || hiveBusy} onClick={() => void loadHive()}>
          {hiveBusy ? "Checking\u2026" : "Check company tabs"}
        </Button>
        <ul className="mt-3 space-y-2 text-sm">
          {(hive?.tabs ?? []).map((tab) => (
            <li key={tab.kind} className="flex items-center justify-between gap-2">
              <span>
                <span className="font-medium">{tab.tab}</span>
                <span className="ml-2 text-ink-muted">{tab.present ? (tab.writable ? "Ready" : "Read only") : tab.error || "Missing"}</span>
              </span>
              {owner && !tab.present && tab.writable ? (
                <Button size="sm" variant="secondary" disabled={busy} onClick={() => void run(() => bootstrapHiveTab(tab.kind), `${tab.tab} tab created.`)}>Create tab</Button>
              ) : null}
            </li>
          ))}
        </ul>
      </section>
      <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:flex-wrap">
        <Button disabled={busy} onClick={() => void run(() => backupData(), "Backup saved.")}>Backup Data</Button>
        <Button variant="secondary" disabled={busy} onClick={() => void chooseRestore()}>Restore</Button>
        <Button variant="secondary" disabled={busy} onClick={() => void run(() => exportLogs(), "Logs exported.")}>Export logs</Button>
        <Button variant="ghost" disabled={busy} onClick={() => setConfirm("logs")}>Clear logs</Button>
      </div>
      <input ref={fileRef} type="file" accept=".zip" className="hidden" onChange={(e) => { const file = e.target.files?.[0]; if (!file) return; setRestoreFile(file); setRestorePath(file.name); setConfirm("restore"); }} />
      {confirm === "restore" ? (
        <Confirm title="Replace the books on this computer?" body="A safety copy is taken first." confirmLabel="Restore" busy={busy} onCancel={() => { setConfirm(null); setRestoreFile(null); setRestorePath(""); }} onConfirm={() => { void run(() => restoreData(restorePath, restoreFile ?? undefined), "Restored.").then(() => { setConfirm(null); setRestoreFile(null); setRestorePath(""); }); }} />
      ) : null}
      {confirm === "logs" ? (
        <Confirm title="Clear logs?" body="Logs will be emptied. Books are not touched." confirmLabel="Clear logs" busy={busy} onCancel={() => setConfirm(null)} onConfirm={() => { void run(() => clearLogs(), "Logs cleared.").then(() => setConfirm(null)); }} />
      ) : null}
      {toast ? <RefreshToast message={toast} onDone={() => setToast(null)} /> : null}
    </ModuleFrame>
  );
}

function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-ink-subtle">{label}</dt>
      <dd className="mt-1 break-all text-ink">{value || "\u2014"}</dd>
    </div>
  );
}

function Confirm({
  title, body, confirmLabel, busy, onCancel, onConfirm,
}: {
  title: string; body: string; confirmLabel: string; busy: boolean; onCancel: () => void; onConfirm: () => void;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/40 p-4">
      <div role="dialog" aria-modal="true" className="enter w-full max-w-md rounded-lg bg-paper-raised p-6 ring-1 ring-line">
        <h2 className="text-2xl font-medium tracking-tight">{title}</h2>
        <p className="mt-3 text-sm text-ink-muted">{body}</p>
        <div className="mt-5 flex justify-end gap-2">
          <Button variant="secondary" onClick={onCancel} disabled={busy}>Cancel</Button>
          <Button variant="danger" onClick={onConfirm} disabled={busy}>{confirmLabel}</Button>
        </div>
      </div>
    </div>
  );
}
