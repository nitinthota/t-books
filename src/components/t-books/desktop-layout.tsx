import { memo, type ReactNode } from "react";
import {
  BookOpen,
  Boxes,
  Building2,
  FileText,
  FolderKanban,
  LayoutDashboard,
  LogOut,
  Package,
  RefreshCw,
  Settings,
  ShoppingCart,
  Truck,
  Users,
  KeyRound,
} from "lucide-react";
import { APP_NAME, APP_VERSION } from "@/lib/t-books/constants";
import { canOpenAccess, canRefresh } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import type { NavId, SearchHit, Session } from "@/lib/t-books/types";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ConflictModal } from "./conflict-modal";
import { GlobalSearch } from "./global-search";
import { Monogram } from "./monogram";
import { RefreshToast } from "./refresh-toast";
import { StatusBanners } from "./status-banners";

export type { NavId };

const NAV: { id: NavId; label: string; icon: typeof LayoutDashboard }[] = [
  { id: "board", label: "Board", icon: LayoutDashboard },
  { id: "vouchers", label: "Vouchers", icon: BookOpen },
  { id: "vendors", label: "Vendors", icon: Building2 },
  { id: "projects", label: "Projects", icon: FolderKanban },
  { id: "sales", label: "Sales", icon: ShoppingCart },
  { id: "purchase", label: "Purchase", icon: Package },
  { id: "hr", label: "HR", icon: Users },
  { id: "inventory", label: "Inventory", icon: Boxes },
  { id: "logistics", label: "Logistics", icon: Truck },
  { id: "documents", label: "Documents", icon: FileText },
  { id: "settings", label: "Settings", icon: Settings },
  { id: "access", label: "Access", icon: KeyRound },
];

export const DesktopLayout = memo(function DesktopLayout({
  session,
  active,
  onNavigate,
  onSignOut,
  onSearchOpen,
  children,
}: {
  session: Session;
  active: NavId;
  onNavigate: (id: NavId) => void;
  onSignOut: () => void;
  onSearchOpen: (id: NavId, hit: SearchHit) => void;
  children: ReactNode;
}) {
  const items = NAV.filter((item) => item.id !== "access" || canOpenAccess(session.role));
  const online = useBooks((s) => s.online);
  const refreshBusy = useBooks((s) => s.refreshBusy);
  const toast = useBooks((s) => s.toast);
  const conflictNumbers = useBooks((s) => s.conflictNumbers);
  const voucherSummary = useBooks((s) => s.voucherSummary);
  const refreshVouchers = useBooks((s) => s.refreshVouchers);
  const forceRefreshVouchers = useBooks((s) => s.forceRefreshVouchers);
  const dismissConflict = useBooks((s) => s.dismissConflict);
  const clearToast = useBooks((s) => s.clearToast);
  const showRefresh = canRefresh(session.role);
  const dirty = voucherSummary?.dirty ?? 0;

  return (
    <div className="flex min-h-dvh bg-paper text-ink">
      <aside className="hidden w-52 shrink-0 flex-col border-r border-line md:flex">
        <div className="flex items-center gap-2.5 px-4 py-5">
          <Monogram className="size-8 rounded-md" />
          <div className="min-w-0">
            <p className="text-[15px] font-medium leading-none tracking-tight">{APP_NAME}</p>
            <p className="mt-1 truncate text-xs text-ink-subtle">This PC · {APP_VERSION}</p>
          </div>
        </div>
        <nav className="flex flex-1 flex-col gap-0.5 overflow-y-auto px-2 pb-3" aria-label="Main">
          {items.map((item) => {
            const Icon = item.icon;
            const isActive = item.id === active;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => onNavigate(item.id)}
                className={cn(
                  "pressable flex h-10 items-center gap-2.5 rounded-md px-2.5 text-left text-sm",
                  isActive
                    ? "bg-navy text-paper-raised"
                    : "text-ink-muted hover:bg-paper-sunken hover:text-ink",
                )}
              >
                <Icon className="size-4 shrink-0" strokeWidth={1.75} aria-hidden="true" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
        <div className="border-t border-line px-2 py-3">
          <p className="mb-2 truncate px-2 text-xs text-ink-subtle">{session.email}</p>
          <Button variant="ghost" size="sm" className="w-full justify-start px-2" onClick={onSignOut}>
            <LogOut className="size-3.5" strokeWidth={1.75} />
            Sign out
          </Button>
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <StatusBanners />
        <header className="flex h-12 items-center gap-3 border-b border-line px-4">
          <GlobalSearch onOpen={onSearchOpen} />
          <div className="ml-auto flex min-w-0 items-center gap-3">
            {showRefresh ? (
              <>
                {dirty > 0 ? (
                  <p className="hidden text-xs text-gold lg:block">{dirty} unsynced</p>
                ) : null}
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={refreshBusy || !online}
                  aria-busy={refreshBusy}
                  onClick={() => void refreshVouchers()}
                >
                  <RefreshCw
                    className={cn("size-3.5", refreshBusy ? "animate-spin" : "")}
                    strokeWidth={1.75}
                    aria-hidden="true"
                  />
                  {refreshBusy ? "Refreshing…" : "Refresh"}
                </Button>
              </>
            ) : null}
          </div>
        </header>
        <nav
          className="flex gap-1 overflow-x-auto border-b border-line px-2 py-2 md:hidden"
          aria-label="Main"
        >
          {items.map((item) => {
            const isActive = item.id === active;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => onNavigate(item.id)}
                className={cn(
                  "pressable h-11 shrink-0 rounded-md px-3 text-sm",
                  isActive ? "bg-navy text-paper-raised" : "text-ink-muted",
                )}
              >
                {item.label}
              </button>
            );
          })}
        </nav>
        <main className="min-w-0 flex-1 overflow-auto">{children}</main>
      </div>
      {conflictNumbers && conflictNumbers.length > 0 ? (
        <ConflictModal
          voucherNumbers={conflictNumbers}
          busy={refreshBusy}
          onKeepLocal={dismissConflict}
          onDiscard={() => void forceRefreshVouchers()}
        />
      ) : null}
      {toast ? <RefreshToast message={toast} onDone={clearToast} /> : null}
    </div>
  );
});
