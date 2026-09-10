import { useEffect } from "react";
import { TriangleAlert } from "lucide-react";
import { useBooks } from "@/lib/t-books/store";
import { BoardShell } from "./board-shell";
import { SignInScreen } from "./sign-in-screen";
import { StatusBanners } from "./status-banners";

export function TBooksRoot() {
  const boot = useBooks((s) => s.boot);
  const bootError = useBooks((s) => s.bootError);
  const session = useBooks((s) => s.session);
  const start = useBooks((s) => s.start);

  useEffect(() => {
    void start();
  }, [start]);

  if (boot === "failed") {
    return (
      <div className="flex min-h-dvh flex-col bg-paper text-ink">
        <StatusBanners />
        <div className="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
          <TriangleAlert className="size-8 text-danger" strokeWidth={1.75} aria-hidden="true" />
          <h1 className="text-xl font-medium tracking-tight">Could not open the local books</h1>
          <p className="max-w-md text-sm break-words text-ink-muted">
            {bootError ?? "Unknown error while opening SQLite on this PC."}
          </p>
        </div>
      </div>
    );
  }

  if (session) return <BoardShell />;
  return <SignInScreen />;
}
