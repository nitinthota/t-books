import { create } from "zustand";
import {
  credentialsFound as localCredentialsFound,
  getAccessSnapshot,
  getLastSynced,
  refreshAccess as localRefreshAccess,
} from "./access";
import { inspectEmail, setFirstPassword, signIn } from "./auth";
import { openLocalBooks } from "./db";
import { isBrowserOnline, subscribeOnline } from "./online";
import {
  invokeErrorMessage,
  isTauriRuntime,
  platformForceRefreshVouchers,
  platformGetAccessList,
  platformGetAppStatus,
  platformGetVoucherSummary,
  platformInspect,
  platformLogin,
  platformLogout,
  platformRefreshAccess,
  platformRefreshVouchers,
  platformSetPassword,
} from "./platform";
import type {
  AccessRow,
  AccessSnapshot,
  AuthInspect,
  RefreshOutcome,
  Session,
  VoucherSummary,
} from "./types";
import {
  forceRefreshVouchers as localForceRefresh,
  getVoucherSummary,
  refreshVouchers as localRefreshVouchers,
} from "./vouchers";

type BootState = "cold" | "ready" | "failed";

type BooksState = {
  boot: BootState;
  bootError: string | null;
  session: Session | null;
  online: boolean;
  credentialsFound: boolean;
  lastSynced: string | null;
  lastError: string | null;
  accessRows: AccessRow[];
  start: () => Promise<void>;
  ensureReady: () => Promise<void>;
  pullStatus: () => Promise<void>;
  inspect: (email: string) => Promise<AuthInspect>;
  login: (email: string, password: string) => Promise<{ ok: true } | { ok: false; message: string }>;
  setPassword: (
    email: string,
    password: string,
    confirm: string,
  ) => Promise<{ ok: true } | { ok: false; message: string }>;
  refreshAccess: () => Promise<{ ok: true } | { ok: false; message: string }>;
  loadAccessList: () => Promise<void>;
  voucherSummary: VoucherSummary | null;
  refreshBusy: boolean;
  toast: string | null;
  conflictNumbers: number[] | null;
  refreshVouchers: () => Promise<{ ok: true } | { ok: false; message: string } | { ok: "dirty" }>;
  forceRefreshVouchers: () => Promise<{ ok: true } | { ok: false; message: string }>;
  dismissConflict: () => void;
  clearToast: () => void;
  loadVoucherSummary: () => Promise<void>;
  logout: () => void;
};

let startOnce: Promise<void> | null = null;
let onlineUnsub: (() => void) | null = null;

function asRole(role: string): Session["role"] {
  if (role === "owner" || role === "admin" || role === "operator") return role;
  return "operator";
}

function applySnapshot(
  set: (partial: Partial<BooksState>) => void,
  snap: AccessSnapshot,
  lastError?: string | null,
) {
  set({
    accessRows: snap.rows,
    lastSynced: snap.lastSynced,
    credentialsFound: snap.credentialsFound,
    ...(lastError !== undefined ? { lastError } : {}),
  });
}

export const useBooks = create<BooksState>((set, get) => ({
  boot: "cold",
  bootError: null,
  session: null,
  online: true,
  credentialsFound: false,
  lastSynced: null,
  lastError: null,
  accessRows: [],
  voucherSummary: null,
  refreshBusy: false,
  toast: null,
  conflictNumbers: null,
  start: async () => {
    if (!startOnce) {
      startOnce = (async () => {
        try {
          if (!isTauriRuntime()) {
            await openLocalBooks();
          }
          if (!onlineUnsub) {
            onlineUnsub = subscribeOnline((online) => set({ online }));
          }
          set({ boot: "ready", bootError: null, online: isBrowserOnline() });
          if (isTauriRuntime()) {
            const status = await platformGetAppStatus();
            const snap = await platformGetAccessList();
            set({
              online: isBrowserOnline(),
              credentialsFound: status.credentialsFound,
              lastSynced: status.lastSynced,
              lastError: status.lastError,
            });
            applySnapshot(set, snap);
            try {
              const summary = await platformGetVoucherSummary();
              set({ voucherSummary: summary });
            } catch {
              /* board stays empty until Refresh */
            }
          } else {
            applySnapshot(set, getAccessSnapshot());
            set({
              online: isBrowserOnline(),
              credentialsFound: localCredentialsFound(),
              lastSynced: getLastSynced(),
              voucherSummary: getVoucherSummary(),
            });
          }
        } catch (err) {
          const message =
            err instanceof Error ? err.message : "Could not open the local books on this PC.";
          set({ boot: "failed", bootError: message, session: null });
          throw err;
        }
      })();
    }
    try {
      await startOnce;
    } catch {
      startOnce = null;
    }
  },
  ensureReady: async () => {
    await get().start();
    if (get().boot !== "ready") {
      throw new Error(get().bootError ?? "Could not open the local books on this PC.");
    }
  },
  pullStatus: async () => {
    try {
      if (isTauriRuntime()) {
        const status = await platformGetAppStatus();
        set({
          online: isBrowserOnline(),
          credentialsFound: status.credentialsFound,
          lastSynced: status.lastSynced,
          lastError: status.lastError,
        });
        return;
      }
      set({
        online: isBrowserOnline(),
        credentialsFound: localCredentialsFound(),
        lastSynced: getLastSynced(),
      });
    } catch {
      set({ online: isBrowserOnline(), credentialsFound: false });
    }
  },
  inspect: async (email) => {
    if (isTauriRuntime()) return platformInspect(email);
    if (get().boot !== "ready") {
      return { kind: "denied", message: "Enter a valid email." };
    }
    return inspectEmail(email);
  },
  login: async (email, password) => {
    try {
      await get().ensureReady();
      if (isTauriRuntime()) {
        const raw = await platformLogin(email, password);
        set({ session: { email: raw.email, role: asRole(raw.role) } });
        await get().pullStatus();
        await get().loadAccessList();
        return { ok: true };
      }
      const result = await signIn(email, password);
      if (!result.ok) {
        await get().pullStatus();
        return result;
      }
      set({ session: result.session });
      await get().pullStatus();
      await get().loadAccessList();
      return { ok: true };
    } catch (err) {
      await get().pullStatus();
      return { ok: false, message: invokeErrorMessage(err) };
    }
  },
  setPassword: async (email, password, confirm) => {
    try {
      await get().ensureReady();
      if (isTauriRuntime()) {
        const raw = await platformSetPassword(email, password, confirm);
        set({ session: { email: raw.email, role: asRole(raw.role) } });
        await get().pullStatus();
        await get().loadAccessList();
        return { ok: true };
      }
      const result = await setFirstPassword(email, password, confirm);
      if (!result.ok) {
        await get().pullStatus();
        return result;
      }
      set({ session: result.session });
      await get().pullStatus();
      await get().loadAccessList();
      return { ok: true };
    } catch (err) {
      await get().pullStatus();
      return { ok: false, message: invokeErrorMessage(err) };
    }
  },
  refreshAccess: async () => {
    try {
      await get().ensureReady();
      if (isTauriRuntime()) {
        try {
          const snap = await platformRefreshAccess();
          applySnapshot(set, snap, null);
          return { ok: true };
        } catch (err) {
          const message = invokeErrorMessage(err);
          set({ lastError: message });
          try {
            const snap = await platformGetAccessList();
            applySnapshot(set, snap);
          } catch {
            /* keep existing rows */
          }
          return { ok: false, message };
        }
      }
      try {
        const snap = await localRefreshAccess();
        applySnapshot(set, snap, null);
        return { ok: true };
      } catch (err) {
        const message = invokeErrorMessage(err);
        set({ lastError: message });
        applySnapshot(set, getAccessSnapshot());
        return { ok: false, message };
      }
    } catch (err) {
      const message = invokeErrorMessage(err);
      set({ lastError: message });
      return { ok: false, message };
    }
  },
  loadAccessList: async () => {
    if (get().boot !== "ready") {
      await get().ensureReady();
    }
    try {
      if (isTauriRuntime()) {
        const snap = await platformGetAccessList();
        applySnapshot(set, snap);
        return;
      }
      applySnapshot(set, getAccessSnapshot());
    } catch (err) {
      set({ lastError: invokeErrorMessage(err) });
    }
  },
  loadVoucherSummary: async () => {
    try {
      if (isTauriRuntime()) {
        set({ voucherSummary: await platformGetVoucherSummary() });
        return;
      }
      set({ voucherSummary: getVoucherSummary() });
    } catch {
      /* keep last summary */
    }
  },
  refreshVouchers: async () => {
    try {
      await get().ensureReady();
      set({ refreshBusy: true, conflictNumbers: null });
      const run = isTauriRuntime() ? platformRefreshVouchers : async () => localRefreshVouchers();
      const result: RefreshOutcome = await run();
      if (result.kind === "dirty") {
        set({ refreshBusy: false, conflictNumbers: result.voucherNumbers });
        return { ok: "dirty" as const };
      }
      await get().loadVoucherSummary();
      const emptyWarning = !result.imported && Boolean(result.warning);
      set({
        refreshBusy: false,
        toast: emptyWarning ? null : "Data refreshed",
        lastError: result.warning,
        conflictNumbers: null,
      });
      return { ok: true as const };
    } catch (err) {
      const message = invokeErrorMessage(err);
      set({ refreshBusy: false, lastError: message });
      try {
        await get().loadVoucherSummary();
      } catch {
        /* keep */
      }
      return { ok: false as const, message };
    }
  },
  forceRefreshVouchers: async () => {
    try {
      await get().ensureReady();
      set({ refreshBusy: true, conflictNumbers: null });
      const run = isTauriRuntime()
        ? platformForceRefreshVouchers
        : async () => localForceRefresh();
      const result: RefreshOutcome = await run();
      if (result.kind === "dirty") {
        set({ refreshBusy: false, conflictNumbers: result.voucherNumbers });
        return { ok: false as const, message: "Unsynced local changes are still on this PC." };
      }
      await get().loadVoucherSummary();
      const emptyWarning = !result.imported && Boolean(result.warning);
      set({
        refreshBusy: false,
        toast: emptyWarning ? null : "Data refreshed",
        lastError: result.warning,
        conflictNumbers: null,
      });
      return { ok: true as const };
    } catch (err) {
      const message = invokeErrorMessage(err);
      set({ refreshBusy: false, lastError: message });
      return { ok: false as const, message };
    }
  },
  dismissConflict: () => set({ conflictNumbers: null, refreshBusy: false }),
  clearToast: () => set({ toast: null }),
  logout: () => {
    if (isTauriRuntime()) {
      void platformLogout();
    }
    set({ session: null });
  },
}));
