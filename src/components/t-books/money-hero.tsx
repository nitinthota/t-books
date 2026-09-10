import { memo } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { cn } from "@/lib/utils";

export const MoneyHero = memo(function MoneyHero({
  value,
  paid,
  remaining,
}: {
  value: number;
  paid: number;
  remaining: number;
}) {
  return (
    <div className="grid grid-cols-3 gap-4">
      <MoneyCell label="Value" amount={value} />
      <MoneyCell label="Paid" amount={paid} />
      <MoneyCell label="Still to pay" amount={remaining} emphasis />
    </div>
  );
});

function MoneyCell({
  label,
  amount,
  emphasis,
}: {
  label: string;
  amount: number;
  emphasis?: boolean;
}) {
  return (
    <div>
      <p className="text-xs text-ink-subtle">{label}</p>
      <p
        className={cn(
          "money-figure mt-1 text-2xl",
          emphasis ? "text-navy" : "text-ink",
        )}
      >
        {formatRupees(amount)}
      </p>
    </div>
  );
}
