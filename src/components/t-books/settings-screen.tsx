import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  bootstrapHiveTab,
  hiveStatus,
  listPendingSubmit,
  retryPendingSubmit,
  type HiveStatus,
} from "@/lib/t-books/control";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { canOpenAccess } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import type { PendingSubmit } from "@/lib/t-books/types";
import {
  backupData,
  checkForUpdates,
  clearLogs,
  downloadUpdate,
  exportLogs,
  getOpsInfo,
  pickRestoreZip,
  restoreData,
  setAutoBackup,
  type OpsInfo,
  type UpdateInfo,
} from "@/lib/t-books/ops";
import { ModuleFrame } from "./module-frame";
import { RefreshToast } from "./refresh-toast";

export default function SettingsScreen({ onBack }: { onBack: () => void }) {
  const [info, setInfo] = useState<OpsInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState<"restore" | "logs" | null>(null);
  const [restorePath, setRestorePath] = useState<string>("");
  const [restoreFile, setRestoreFile] = useState<File | null>(null);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [hive, setHive] = useState<HiveStatus | null>(null);
  const [pending, setPending] = useState<PendingSubmit[]>([]);
  const fileRef = useRef<HTMLInputElement>(null);
  const session = useBooks((s) => s.session);
  const owner = session ? canOpenAccess(session.role) : false;

  const load = useCallback(async () => {
    try {
      setInfo(await getOpsInfo());
      try {
        setHive(await hiveStatus());
        setPending(await listPendingSubmit());
      } catch {
        setHive(null);
        setPending([]);
      }
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function run(work: () => Promise<string | void>, ok: string) {
    setBusy(true);
    setError(null);
    try {
      const result = await work();
      setToast(typeof result === "string" && result ? result : ok);
      await load();
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
    <ModuleFrame title="System" hint="Who is signed in on this PC, hive tabs on Google, and data controls." onBack={onBack} error={error}>
      <section className="mb-8 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
        <h2 className="text-lg font-medium">Signed-in operator</h2>
        <dl className="mt-3 grid gap-2 text-sm sm:grid-cols-2">
          <Meta label="Email" value={session?.email ?? "—"} />
          <Meta label="Role" value={session?.role ?? "—"} />
          <Meta label="Version" value={info?.version ?? "…"} />
          <Meta label="Data folder" value={info?.dataDir ?? "…"} />
        </dl>
      </section>

      <section className="mb-8 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
        <h2 className="text-lg font-medium">Windows hive</h2>
        <p className="mt-1 text-sm text-ink-muted">
          Views never call Google. Submit writes one hive row. Owner can create missing tabs (headers only).
        </p>
        <p className="mt-2 text-xs text-ink-subtle">
          {hive
            ? hive.online
              ? hive.credentialsFound
                ? "This PC can reach Google."
                : "Google credentials not found. Running offline."
              : "Offline — hive inspect paused."
            : "Hive status loads on this PC."}
        </p>
        <ul className="mt-3 divide-y divide-line text-sm">
          {(hive?.tabs ?? []).map((tab) => (
            <li key={tab.kind} className="flex flex-wrap items-center justify-between gap-2 py-2">
              <span>
                {tab.tab}
                <span className="ml-2 text-xs text-ink-subtle">
                  {tab.present ? `${tab.rowCount} rows` : "missing"}
                  {tab.error ? ` · ${tab.error}` : ""}
                </span>
              </span>
              {owner && !tab.present && tab.writable !== false ? (
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy}
                  onClick={() => void run(() => bootstrapHiveTab(tab.kind), `${tab.tab} ready.`)}
                >
                  Create headers
                </Button>
              ) : null}
            </li>
          ))}
        </ul>
        {pending.length > 0 ? (
          <div className="mt-4">
            <p className="text-sm font-medium">Outbox</p>
            <ul className="mt-2 space-y-2 text-sm">
              {pending.map((row) => (
                <li key={`${row.kind}:${row.key}`} className="flex items-center justify-between gap-2">
                  <span className="font-mono text-xs">
                    {row.kind} {row.key}
                    {row.lastError ? ` · ${row.lastError}` : ""}
                  </span>
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => {
                        const out = await retryPendingSubmit(row.kind, row.key);
                        if (out.kind === "conflict") {
                          throw new Error(out.message);
                        }
                      }, "Submitted to Google.")
                    }
                  >
                    Retry
                  </Button>
                </li>
              ))}
            </ul>
          </div>
        ) : (
          <p className="mt-3 text-sm text-ink-muted">No pending hive writes on this PC.</p>
        )}
      </section>

      <dl className="grid gap-4 text-sm sm:grid-cols-2">
        <Meta label="Debug logs" value={info?.debug ? "On (--debug)" : "Off"} />
        <Meta label="Auto backup" value={info?.autoBackup ? "On close, last 5" : "Off"} />
      </dl>

      <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:flex-wrap">
        <Button disabled={busy} onClick={() => void run(() => backupData(), "Backup saved.")}>
          Backup Data
        </Button>
        <Button variant="secondary" disabled={busy} onClick={() => void chooseRestore()}>
          Restore
        </Button>
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() =>
            void run(async () => {
              const found = await checkForUpdates();
              setUpdate(found);
              return found.newer
                ? `Version ${found.latest} is available.`
                : "You are on the latest T Books.";
            }, "Checked.")
          }
        >
          Check for updates
        </Button>
        <Button variant="secondary" disabled={busy} onClick={() => void run(() => exportLogs(), "Logs exported.")}>
          Export logs
        </Button>
        <Button variant="ghost" disabled={busy} onClick={() => setConfirm("logs")}>
          Clear logs
        </Button>
      </div>

      <label className="mt-6 flex items-center gap-2 text-sm text-ink">
        <input
          type="checkbox"
          className="size-4 accent-[var(--color-navy)]"
          checked={info?.autoBackup ?? true}
          disabled={busy || !info}
          onChange={(e) => {
            const on = e.target.checked;
            void run(async () => {
              await setAutoBackup(on);
            }, on ? "Auto backup on close." : "Auto backup off.");
          }}
        />
        Auto backup on close (keeps last 5)
      </label>

      {update?.newer && update.downloadUrl ? (
        <div className="mt-6 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
          <p className="text-sm text-ink">
            {update.latest} is newer than {update.current}. Download T-Books-Setup.exe and install it
            yourself. Your books stay in the data folder.
          </p>
          <Button
            className="mt-3"
            disabled={busy}
            onClick={() =>
              void run(() => downloadUpdate(update.downloadUrl!), "Installer saved. Run it when ready.")
            }
          >
            Download installer
          </Button>
        </div>
      ) : null}

      <input
        ref={fileRef}
        type="file"
        accept=".zip"
        className="hidden"
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (!file) return;
          setRestoreFile(file);
          setRestorePath(file.name);
          setConfirm("restore");
        }}
      />

      {confirm === "restore" ? (
        <Confirm
          title="Replace the books on this PC?"
          body="A safety backup of the current file is made first. This cannot be undone except by restoring that safety copy."
          confirmLabel="Restore"
          busy={busy}
          onCancel={() => {
            setConfirm(null);
            setRestoreFile(null);
            setRestorePath("");
          }}
          onConfirm={() => {
            void run(
              () => restoreData(restorePath, restoreFile ?? undefined),
              "Restored.",
            ).then(() => {
              setConfirm(null);
              setRestoreFile(null);
              setRestorePath("");
            });
          }}
        />
      ) : null}

      {confirm === "logs" ? (
        <Confirm
          title="Clear logs on this PC?"
          body="Error and performance logs will be emptied. Books are not touched."
          confirmLabel="Clear logs"
          busy={busy}
          onCancel={() => setConfirm(null)}
          onConfirm={() => {
            void run(() => clearLogs(), "Logs cleared.").then(() => setConfirm(null));
          }}
        />
      ) : null}

      {toast ? <RefreshToast message={toast} onDone={() => setToast(null)} /> : null}
    </ModuleFrame>
  );
}

function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-ink-subtle">{label}</dt>
      <dd className="mt-1 break-all text-ink">{value || "—"}</dd>
    </div>
  );
}

function Confirm({
  title,
  body,
  confirmLabel,
  busy,
  onCancel,
  onConfirm,
}: {
  title: string;
  body: string;
  confirmLabel: string;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/40 p-4">
      <div role="dialog" aria-modal="true" className="enter w-full max-w-md rounded-lg bg-paper-raised p-6 ring-1 ring-line">
        <h2 className="text-2xl font-medium tracking-tight">{title}</h2>
        <p className="mt-3 text-sm text-ink-muted">{body}</p>
        <div className="mt-5 flex justify-end gap-2">
          <Button variant="secondary" onClick={onCancel} disabled={busy}>
            Cancel
          </Button>
          <Button variant="danger" onClick={onConfirm} disabled={busy}>
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
