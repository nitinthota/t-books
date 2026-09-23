import { useCallback, useState } from "react";
import PurchaseDesk from "./purchase-desk";
import PurchaseScreen from "./purchase-screen";

export default function PurchaseGate({
  onBack,
  focusId,
  onOpenProject,
  onOpenVendor,
}: {
  onBack: () => void;
  focusId?: string | null;
  onOpenProject?: (project: string) => void;
  onOpenVendor?: (vendor: string) => void;
}) {
  const [editId, setEditId] = useState<string | null>(null);
  const onEdit = useCallback((id: number) => setEditId(String(id)), []);
  const onNew = useCallback(() => setEditId("new"), []);

  if (editId && editId !== "new") {
    return <PurchaseScreen onBack={() => setEditId(null)} focusId={editId} />;
  }
  if (editId === "new") {
    return <PurchaseScreen onBack={() => setEditId(null)} />;
  }

  return (
    <PurchaseDesk
      onBack={onBack}
      focusId={focusId}
      onOpenProject={onOpenProject}
      onOpenVendor={onOpenVendor}
      onEdit={onEdit}
      onNew={onNew}
    />
  );
}
