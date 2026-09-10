import { memo } from "react";
import { FileWarning, TriangleAlert, WifiOff } from "lucide-react";
import { CREDENTIALS_BANNER, OFFLINE_BANNER } from "@/lib/t-books/constants";
import { useBooks } from "@/lib/t-books/store";

export const StatusBanners = memo(function StatusBanners() {
  const online = useBooks((s) => s.online);
  const credentialsFound = useBooks((s) => s.credentialsFound);
  const lastError = useBooks((s) => s.lastError);

  return (
    <div className="flex flex-col">
      {!online ? (
        <div
          role="status"
          className="flex items-start gap-2 border-b border-line bg-offline-bg px-4 py-2 text-sm text-offline"
        >
          <WifiOff className="mt-0.5 size-4 shrink-0" strokeWidth={1.75} aria-hidden="true" />
          <p>{OFFLINE_BANNER}</p>
        </div>
      ) : null}
      {!credentialsFound ? (
        <div
          role="status"
          className="flex items-start gap-2 border-b border-line bg-navy-soft px-4 py-2 text-sm text-navy"
        >
          <FileWarning className="mt-0.5 size-4 shrink-0" strokeWidth={1.75} aria-hidden="true" />
          <p>{CREDENTIALS_BANNER}</p>
        </div>
      ) : null}
      {lastError ? (
        <div
          role="alert"
          className="flex items-start gap-2 border-b border-line bg-danger-bg px-4 py-2 text-sm text-danger-fg"
        >
          <TriangleAlert className="mt-0.5 size-4 shrink-0" strokeWidth={1.75} aria-hidden="true" />
          <p className="break-words">{lastError}</p>
        </div>
      ) : null}
    </div>
  );
});
