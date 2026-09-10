import { memo } from "react";
import { TriangleAlert } from "lucide-react";

export const ErrorBanner = memo(function ErrorBanner({ message }: { message: string }) {
  return (
    <div
      role="alert"
      className="flex items-start gap-2 rounded-md bg-danger-bg px-3 py-2.5 text-sm text-danger-fg"
    >
      <TriangleAlert className="mt-0.5 size-4 shrink-0" strokeWidth={1.75} aria-hidden="true" />
      <p className="break-words">{message}</p>
    </div>
  );
});
