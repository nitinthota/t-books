import { lazy, Suspense, useCallback, useState } from "react";
import { canOpenAccess } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import type { NavId, SearchHit } from "@/lib/t-books/types";
import { BoardHome } from "./board-home";
import { DesktopLayout } from "./desktop-layout";
import { ScreenSkeleton } from "./skeleton";
import { UnsavedProvider, useUnsaved } from "./unsaved-guard";

const VouchersScreen = lazy(() => import("./vouchers-screen"));
const VendorsScreen = lazy(() => import("./vendors-screen"));
const ProjectsScreen = lazy(() => import("./projects-screen"));
const SalesScreen = lazy(() => import("./sales-gate"));
const PurchaseScreen = lazy(() => import("./purchase-gate"));
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
            <BoardHome onOpen={(nav) => go(nav)} />
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
              <SalesScreen
                onBack={() => go("board")}
                focusId={focusId}
                onOpenProject={(project) =>
                  go("projects", { kind: "project", id: project, title: project, subtitle: "" })
                }
              />
            ) : active === "purchase" ? (
              <PurchaseScreen
                onBack={() => go("board")}
                focusId={focusId}
                onOpenProject={(project) =>
                  go("projects", { kind: "project", id: project, title: project, subtitle: "" })
                }
                onOpenVendor={(vendor) =>
                  go("vendors", { kind: "vendor", id: vendor, title: vendor, subtitle: "" })
                }
              />
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
