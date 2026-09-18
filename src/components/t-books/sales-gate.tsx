import { useCallback, useState } from "react";
import type { NavId } from "@/lib/t-books/types";
import SalesDesk from "./sales-desk";
import SalesScreen from "./sales-screen";

export default function SalesGate({
  onBack,
  focusId,
  onOpenProject,
}: {
  onBack: () => void;
  focusId?: string | null;
  onOpenProject?: (project: string) => void;
}) {
  const [editId, setEditId] = useState<string | null>(null);

  const onEdit = useCallback((id: number) => setEditId(String(id)), []);
  const onNew = useCallback(() => setEditId("new"), []);

  if (editId && editId !== "new") {
    return <SalesScreen onBack={() => setEditId(null)} focusId={editId} />;
  }
  if (editId === "new") {
    return <SalesScreen onBack={() => setEditId(null)} />;
  }

  return (
    <SalesDesk
      onBack={onBack}
      focusId={focusId}
      onOpenProject={onOpenProject}
      onEdit={onEdit}
      onNew={onNew}
    />
  );
}

export type SalesGateNav = NavId;
