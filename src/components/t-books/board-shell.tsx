import { lazy, memo, Suspense, useCallback, useEffect, useState } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { canOpenAccess } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import type { DirtyKey, NavId, SearchHit } from "@/lib/t-books/types";
import { invokeCommand, isTauriRuntime } from "@/lib/t-books/platform";
import { listDirtyKeys } from "@/lib/t-books/vouchers";
import { DesktopLayout } from "./desktop-layout";
import { ScreenSkeleton } from "./skeleton";
import { UnsavedProvider, useUnsaved } from "./unsaved-guard";

const VouchersScreen = lazy(() => import("./vouchers-screen"));
const VendorsScreen = lazy(() => import("./vendors-screen"));
const ProjectsScreen = lazy(() => import("./projects-screen"));
const SalesScreen = lazy(() => import("./sales-screen"));
const PurchaseScreen = lazy(() => import("./purchase-screen"));
const HrScreen = lazy(() => import("./hr-screen"));
const TrialScreen = lazy(() => import("./trial-screen"));
const InventoryScreen = lazy(() => import("./inventory-screen"));
const LogisticsScreen = lazy(() => import("./logistics-screen"));
const DocumentsScreen = lazy(() => import("./documents-screen"));
const DuplicatesScreen = lazy(() => import("./duplicates-screen"));
const MergeScreen = lazy(() => import("./merge-screen"));
const ExplorerScreen = lazy(() => import("./explorer-screen"));
const RulesScreen = lazy(() => import("./rules-screen"));
const SettingsScreen = lazy(() => import("./settings-screen"));
const AccessScreen = lazy(() =>
  import("./access-screen").then((m) => ({ default: m.AccessScreen })),
);

export function BoardShell() {
  return (
    <UnsavedProvider>
      <BoardShellInner />
    </UnsavedProvider>
  );
}

function BoardShellInner() {
  const session = useBooks((s) => s.session);
  const logout = useBooks((s) => s.logout);
  const { requestLeave } = useUnsaved();
  const [active, setActive] = useState<NavId>("board");
  const [focusId, setFocusId] = useState<string | null>(null);

  const go = useCallback(
    (id: NavId, hit?: SearchHit) => {
      if (id === "access" && session && !canOpenAccess(session.role)) return;
      requestLeave(() => {
        setFocusId(hit?.id ?? null);
        setActive(id);
      });
    },
    [requestLeave, session],
  );

  if (!session) return null;

  return (
    <DesktopLayout
      session={session}
      active={active}
      onNavigate={(id) => go(id)}
      onSearchOpen={(id, hit) => go(id, hit)}
      onSignOut={() => requestLeave(logout)}
    >
      <div key={active} className="mx-auto w-full max-w-6xl px-6 py-7">
        {active === "board" ? (
          <section className="enter">
            <h1 className="text-3xl font-medium tracking-tight">Board</h1>
            <BoardBody onOpenVouchers={() => go("vouchers")} />
          </section>
        ) : (
          <Suspense fallback={<ScreenSkeleton />}>
            {active === "vouchers" ? (
              <VouchersScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "vendors" ? (
              <VendorsScreen
                onBack={() => go("board")}
                focusId={focusId}
                onOpen={(nav, key) =>
                  go(nav, {
                    kind: nav === "vendors" ? "vendor" : "project",
                    id: key,
                    title: key,
                    subtitle: "",
                  })
                }
              />
            ) : active === "projects" ? (
              <ProjectsScreen
                onBack={() => go("board")}
                focusId={focusId}
                onOpen={(nav, key) =>
                  go(nav, {
                    kind: nav === "vendors" ? "vendor" : "project",
                    id: key,
                    title: key,
                    subtitle: "",
                  })
                }
              />
            ) : active === "sales" ? (
              <SalesScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "purchase" ? (
              <PurchaseScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "hr" ? (
              <HrScreen onBack={() => go("board")} />
            ) : active === "finance" ? (
              <TrialScreen onBack={() => go("board")} />
            ) : active === "inventory" ? (
              <InventoryScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "logistics" ? (
              <LogisticsScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "documents" ? (
              <DocumentsScreen onBack={() => go("board")} />
            ) : active === "duplicates" ? (
              <DuplicatesScreen onBack={() => go("board")} />
            ) : active === "merge" ? (
              <MergeScreen onBack={() => go("board")} />
            ) : active === "explorer" ? (
              <ExplorerScreen onBack={() => go("board")} />
            ) : active === "rules" ? (
              <RulesScreen onBack={() => go("board")} />
            ) : active === "system" ? (
              <SettingsScreen onBack={() => go("board")} />
            ) : active === "access" ? (
              <AccessScreen onBack={() => go("board")} />
            ) : null}
          </Suspense>
        )}
      </div>
    </DesktopLayout>
  );
}

const BoardBody = memo(function BoardBody({ onOpenVouchers }: { onOpenVouchers: () => void }) {
  const summary = useBooks((s) => s.voucherSummary);
  const lastError = useBooks((s) => s.lastError);
  const count = summary?.count ?? 0;
  const dirty = summary?.dirty ?? 0;
  const remaining = summary?.remainingTotal ?? 0;
  const [keys, setKeys] = useState<DirtyKey[]>([]);

  useEffect(() => {
    void (async () => {
      try {
        const list = isTauriRuntime()
          ? await invokeCommand<DirtyKey[]>("get_dirty_keys")
          : listDirtyKeys();
        setKeys(list);
      } catch {
        /* board still shows last summary */
      }
    })();
  }, [summary, lastError]);

  return (
    <div className="mt-8">
      {lastError ? (
        <p className="mb-4 text-sm text-gold">Refresh failed. Previous figures are kept.</p>
      ) : null}
      {count === 0 && !lastError ? (
        <p className="text-sm text-ink-muted">No vouchers yet. Refresh loads the register.</p>
      ) : (
        <>
          <p className="text-xs text-ink-subtle">
            {summary?.lastSynced ? `Last refreshed ${summary.lastSynced}` : "Not refreshed yet."}
          </p>
          <button
            type="button"
            onClick={onOpenVouchers}
            className="board-card pressable mt-4 w-full rounded-lg bg-paper-raised p-5 text-left ring-1 ring-line"
          >
            <div className="grid grid-cols-3 gap-4">
              <BoardFigure label="Vouchers" value={String(count)} />
              <BoardFigure label="Still to pay" value={formatRupees(remaining)} emphasis />
              <BoardFigure label="Unsynced" value={String(keys.length || dirty)} gold={(keys.length || dirty) > 0} />
            </div>
          </button>
        </>
      )}
      {keys.length > 0 ? (
        <div className="mt-6 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
          <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Not posted</p>
          <ul className="mt-2 max-h-40 overflow-auto text-sm">
            {keys.map((k) => (
              <li key={`${k.kind}-${k.key}`} className="py-1">
                {k.kind} {k.key}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
});

function BoardFigure({
  label,
  value,
  emphasis,
  gold,
}: {
  label: string;
  value: string;
  emphasis?: boolean;
  gold?: boolean;
}) {
  return (
    <div>
      <p className="text-xs text-ink-subtle">{label}</p>
      <p
        className={
          gold
            ? "money-figure mt-1 text-3xl text-gold"
            : emphasis
              ? "money-figure mt-1 text-3xl text-navy"
              : "money-figure mt-1 text-3xl"
        }
      >
        {value}
      </p>
    </div>
  );
}
