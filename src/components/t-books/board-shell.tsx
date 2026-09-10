import { lazy, memo, Suspense, useCallback, useState } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { canOpenAccess } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import type { NavId, SearchHit } from "@/lib/t-books/types";
import { DesktopLayout } from "./desktop-layout";
import { ScreenSkeleton } from "./skeleton";
import { UnsavedProvider, useUnsaved } from "./unsaved-guard";

const VouchersScreen = lazy(() => import("./vouchers-screen"));
const VendorsScreen = lazy(() => import("./vendors-screen"));
const ProjectsScreen = lazy(() => import("./projects-screen"));
const SalesScreen = lazy(() => import("./sales-screen"));
const PurchaseScreen = lazy(() => import("./purchase-screen"));
const HrScreen = lazy(() => import("./hr-screen"));
const InventoryScreen = lazy(() => import("./inventory-screen"));
const LogisticsScreen = lazy(() => import("./logistics-screen"));
const DocumentsScreen = lazy(() => import("./documents-screen"));
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
      <div className="mx-auto w-full max-w-6xl px-6 py-7">
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
              <VendorsScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "projects" ? (
              <ProjectsScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "sales" ? (
              <SalesScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "purchase" ? (
              <PurchaseScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "hr" ? (
              <HrScreen onBack={() => go("board")} />
            ) : active === "inventory" ? (
              <InventoryScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "logistics" ? (
              <LogisticsScreen onBack={() => go("board")} focusId={focusId} />
            ) : active === "documents" ? (
              <DocumentsScreen onBack={() => go("board")} />
            ) : active === "settings" ? (
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

  if (count === 0) {
    return (
      <div className="mt-6 max-w-xl">
        {lastError ? null : (
          <p className="text-sm text-ink-muted">
            No vouchers on this PC yet. Refresh pulls the register. Sales, purchase, HR, inventory,
            logistics, and documents stay here.
          </p>
        )}
      </div>
    );
  }

  return (
    <div className="mt-8">
      <p className="text-xs text-ink-subtle">
        {summary?.lastSynced ? `Last refreshed ${summary.lastSynced}` : "Never refreshed on this PC."}
      </p>
      <button
        type="button"
        onClick={onOpenVouchers}
        className="pressable mt-4 w-full rounded-lg bg-paper-raised p-5 text-left ring-1 ring-line"
      >
        <div className="grid grid-cols-3 gap-4">
          <BoardFigure label="Vouchers" value={String(count)} />
          <BoardFigure label="Still to pay" value={formatRupees(remaining)} emphasis />
          <BoardFigure
            label="Unsynced"
            value={String(dirty)}
            gold={dirty > 0}
          />
        </div>
      </button>
      <p className="mt-4 text-sm text-ink-muted">
        {summary?.pending ?? 0} pending · {summary?.partial ?? 0} partial · {summary?.full ?? 0} full
        {(summary?.missingTax ?? 0) > 0 ? ` · ${summary?.missingTax} missing tax invoice` : ""}
      </p>
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
