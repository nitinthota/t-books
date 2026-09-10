import type { ErrorComponentProps } from "@tanstack/react-router";
import { TriangleAlert } from "lucide-react";

const FALLBACK_MESSAGE = "An unexpected error occurred. Try reloading T Books.";

function errorMessage(error: unknown): string {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string" && error) return error;
  return FALLBACK_MESSAGE;
}

export function AppErrorComponent({ error }: ErrorComponentProps) {
  return (
    <main className="flex min-h-dvh flex-col items-center justify-center gap-3 bg-paper px-6 text-center text-ink">
      <span className="text-danger" aria-hidden="true">
        <TriangleAlert className="size-10" strokeWidth={1.75} />
      </span>
      <h1 className="text-lg font-medium tracking-tight">Something went wrong</h1>
      <p className="max-w-md text-sm break-words text-ink-muted">{errorMessage(error)}</p>
    </main>
  );
}
