import { APP_MONOGRAM } from "@/lib/t-books/constants";
import { cn } from "@/lib/utils";

export function Monogram({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "grid size-11 place-items-center rounded-lg bg-navy text-paper-raised",
        className,
      )}
      aria-hidden="true"
    >
      <span className="font-display text-xl font-medium leading-none tracking-tight">{APP_MONOGRAM}</span>
    </div>
  );
}
