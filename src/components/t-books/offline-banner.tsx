import { WifiOff } from "lucide-react";
import { OFFLINE_BANNER } from "@/lib/t-books/constants";
import { useBooks } from "@/lib/t-books/store";

export function OfflineBanner() {
  const online = useBooks((s) => s.online);
  if (online) return null;
  return (
    <div
      role="status"
      className="flex items-start gap-2 border-b border-line bg-offline-bg px-4 py-2.5 text-sm text-offline"
    >
      <WifiOff className="mt-0.5 size-4 shrink-0" strokeWidth={1.75} aria-hidden="true" />
      <p>{OFFLINE_BANNER}</p>
    </div>
  );
}
