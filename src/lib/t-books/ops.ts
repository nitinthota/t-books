import { APP_VERSION } from "./constants";
import { exportDbBytes, getMeta, replaceFromBytes, run } from "./db";
import { invokeCommand, isTauriRuntime } from "./platform";
import { unzipNamed, zipStore } from "./zip";

export type OpsInfo = {
  version: string;
  dataDir: string;
  dbPath: string;
  autoBackup: boolean;
  debug: boolean;
};

export type UpdateInfo = {
  current: string;
  latest: string;
  newer: boolean;
  notes: string;
  downloadUrl: string | null;
};

function downloadBlob(name: string, bytes: Uint8Array) {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  const blob = new Blob([copy], { type: "application/zip" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  URL.revokeObjectURL(url);
}

export async function getOpsInfo(): Promise<OpsInfo> {
  if (isTauriRuntime()) return invokeCommand<OpsInfo>("get_ops_info");
  return {
    version: getMeta("version") || APP_VERSION,
    dataDir: "This browser (IndexedDB)",
    dbPath: "tbooks.db",
    autoBackup: getMeta("auto_backup") !== "no",
    debug: false,
  };
}

export async function backupData(): Promise<string> {
  if (isTauriRuntime()) return invokeCommand<string>("backup_data");
  const db = exportDbBytes();
  const zip = zipStore([
    { name: "tbooks.db", data: db },
    {
      name: "manifest.json",
      data: new TextEncoder().encode(`{"app":"T Books","version":"${APP_VERSION}"}`),
    },
  ]);
  downloadBlob(`T-Books-backup.zip`, zip);
  return "T-Books-backup.zip";
}

export async function pickRestoreZip(): Promise<string> {
  if (isTauriRuntime()) return invokeCommand<string>("pick_restore_zip");
  return "";
}

export async function restoreData(src: string, file?: File): Promise<string> {
  if (isTauriRuntime()) return invokeCommand<string>("restore_data", { src });
  if (!file) throw new Error("Choose a backup zip on this PC.");
  const buf = new Uint8Array(await file.arrayBuffer());
  const db = unzipNamed(buf, "tbooks.db");
  if (!db) {
    throw new Error("That zip does not contain tbooks.db. Local books were not changed.");
  }
  await replaceFromBytes(db);
  if (typeof window !== "undefined") {
    window.setTimeout(() => window.location.reload(), 400);
  }
  return "Restored.";
}

export async function exportLogs(): Promise<string> {
  if (isTauriRuntime()) return invokeCommand<string>("export_logs");
  throw new Error("Log export is available in the installed T Books app.");
}

export async function clearLogs(): Promise<void> {
  if (isTauriRuntime()) {
    await invokeCommand<void>("clear_logs_cmd");
    return;
  }
}

export async function setAutoBackup(enabled: boolean): Promise<void> {
  if (isTauriRuntime()) {
    await invokeCommand<void>("set_auto_backup_cmd", { enabled });
    return;
  }
  run("INSERT OR REPLACE INTO app_meta (key, value) VALUES ('auto_backup', ?)", [
    enabled ? "yes" : "no",
  ]);
}

export async function checkForUpdates(): Promise<UpdateInfo> {
  if (isTauriRuntime()) return invokeCommand<UpdateInfo>("check_for_updates");
  throw new Error("Update check is available in the installed T Books app.");
}

export async function downloadUpdate(url: string): Promise<string> {
  if (isTauriRuntime()) return invokeCommand<string>("download_update", { url });
  throw new Error("Installer download is available in the installed T Books app.");
}
