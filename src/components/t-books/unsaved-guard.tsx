import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { Button } from "@/components/ui/button";

type SaveFn = () => Promise<void>;

type UnsavedApi = {
  register: (dirty: boolean, save: SaveFn | null) => void;
  requestLeave: (then: () => void) => void;
};

const UnsavedCtx = createContext<UnsavedApi | null>(null);

export function UnsavedProvider({ children }: { children: ReactNode }) {
  const dirtyRef = useRef(false);
  const saveRef = useRef<SaveFn | null>(null);
  const pendingRef = useRef<(() => void) | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const register = useCallback((dirty: boolean, save: SaveFn | null) => {
    dirtyRef.current = dirty;
    saveRef.current = save;
  }, []);

  const requestLeave = useCallback((then: () => void) => {
    if (!dirtyRef.current) {
      then();
      return;
    }
    pendingRef.current = then;
    setActionError(null);
    setOpen(true);
  }, []);

  async function onSave() {
    setBusy(true);
    setActionError(null);
    try {
      if (saveRef.current) await saveRef.current();
      dirtyRef.current = false;
      setOpen(false);
      const next = pendingRef.current;
      pendingRef.current = null;
      next?.();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Could not save on this PC.";
      setActionError(message);
    } finally {
      setBusy(false);
    }
  }

  function onDiscard() {
    dirtyRef.current = false;
    saveRef.current = null;
    setOpen(false);
    const next = pendingRef.current;
    pendingRef.current = null;
    next?.();
  }

  function onCancel() {
    pendingRef.current = null;
    setOpen(false);
  }

  const api = useMemo(() => ({ register, requestLeave }), [register, requestLeave]);

  return (
    <UnsavedCtx.Provider value={api}>
      {children}
      {open ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/40 px-4">
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="unsaved-title"
            className="enter w-full max-w-md rounded-lg bg-paper-raised p-6 ring-1 ring-line"
          >
            <h2 id="unsaved-title" className="text-xl font-medium tracking-tight">
              Unsaved changes
            </h2>
            <p className="mt-2 text-sm text-ink-muted">
              Save them on this PC, discard, or keep editing.
            </p>
            {actionError ? (
              <p className="mt-3 text-sm text-danger-fg" role="alert">
                {actionError}
              </p>
            ) : null}
            <div className="mt-5 flex flex-col gap-2 sm:flex-row">
              <Button onClick={() => void onSave()} disabled={busy}>
                {busy ? "Saving…" : "Save and go back"}
              </Button>
              <Button variant="secondary" onClick={onDiscard} disabled={busy}>
                Discard
              </Button>
              <Button variant="ghost" onClick={onCancel} disabled={busy}>
                Cancel
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </UnsavedCtx.Provider>
  );
}

export function useUnsaved(): UnsavedApi {
  const ctx = useContext(UnsavedCtx);
  if (!ctx) {
    throw new Error("Unsaved guard is not mounted.");
  }
  return ctx;
}

export function useRegisterUnsaved(dirty: boolean, save: SaveFn): void {
  const { register } = useUnsaved();
  useEffect(() => {
    register(dirty, save);
    return () => register(false, null);
  }, [dirty, save, register]);
}
