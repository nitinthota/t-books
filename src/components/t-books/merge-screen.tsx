import { useCallback, useEffect, useState } from "react";
import { listPurchasePo } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { PurchasePo, VoucherListRow } from "@/lib/t-books/types";
import { loadVoucherList } from "@/lib/t-books/vouchers";
import { MergePane } from "./merge-pane";
import { ModuleFrame } from "./module-frame";

export default function MergeScreen({ onBack }: { onBack: () => void }) {
  const [bills, setBills] = useState<PurchasePo[]>([]);
  const [vouchers, setVouchers] = useState<VoucherListRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [po, voucherRows] = await Promise.all([listPurchasePo(), loadVoucherList()]);
      setBills(po);
      setVouchers(voucherRows);
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <ModuleFrame
      title="Merge"
      hint="Link vouchers onto a PUR bill on this PC. Same vendor only. Posted PUR-n / PUR-n-01 / PAY-n are never rewritten. Same class of Control tool as Duplicates."
      onBack={onBack}
      error={error}
    >
      <MergePane bills={bills} vouchers={vouchers} />
    </ModuleFrame>
  );
}
