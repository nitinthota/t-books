import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  listDuplicates,
  mergeMaster,
  type DuplicateGroup,
  type DuplicatesReport,
} from "@/lib/t-books/control";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { ModuleFrame } from "./module-frame";

export default function DuplicatesScreen({ onBack }: { onBack: () => void }) {
  const [data, setData] = useState<DuplicatesReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setData(await listDuplicates());
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function onMerge(kind: "vendor" | "project", keepId: string, absorbIds: string[]) {
    setBusy(true);
    try {
      await mergeMaster(kind, keepId, absorbIds);
      await load();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <ModuleFrame
      title="Duplicates"
      hint="Lookalikes are reported only. Merge is optional — you pick the name to keep. Duplicate voucher numbers are never merged."
      onBack={onBack}
      error={error}
    >
      <Section title="Voucher numbers" empty="No duplicate voucher numbers." count={data?.serials.length ?? 0}>
        {(data?.serials ?? []).map((s) => (
          <li key={s.voucherNo} className="flex justify-between px-4 py-3">
            <span className="font-mono text-sm">{s.voucherNo}</span>
            <span className="text-sm text-ink-muted">{s.count} times · not merged</span>
          </li>
        ))}
      </Section>
      <MergeGroups
        title="Vendor names"
        empty="No lookalike vendor names."
        groups={data?.vendors ?? []}
        busy={busy}
        onMerge={(keep, absorb) => void onMerge("vendor", keep, absorb)}
      />
      <MergeGroups
        title="Project names"
        empty="No lookalike project names."
        groups={data?.projects ?? []}
        busy={busy}
        onMerge={(keep, absorb) => void onMerge("project", keep, absorb)}
      />
    </ModuleFrame>
  );
}

function Section({
  title,
  empty,
  count,
  children,
}: {
  title: string;
  empty: string;
  count: number;
  children: React.ReactNode;
}) {
  return (
    <section className="mb-6 overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
      <h2 className="px-4 py-3 text-sm font-medium">{title}</h2>
      {count === 0 ? <p className="px-4 pb-4 text-sm text-ink-muted">{empty}</p> : <ul>{children}</ul>}
    </section>
  );
}

function MergeGroups({
  title,
  empty,
  groups,
  busy,
  onMerge,
}: {
  title: string;
  empty: string;
  groups: DuplicateGroup[];
  busy: boolean;
  onMerge: (keepId: string, absorbIds: string[]) => void;
}) {
  return (
    <Section title={title} empty={empty} count={groups.length}>
      {groups.map((g) => (
        <MergeRow key={g.key} variants={g.variants} busy={busy} onMerge={onMerge} />
      ))}
    </Section>
  );
}

function MergeRow({
  variants,
  busy,
  onMerge,
}: {
  variants: DuplicateGroup["variants"];
  busy: boolean;
  onMerge: (keepId: string, absorbIds: string[]) => void;
}) {
  const [keepId, setKeepId] = useState(variants[0]?.id ?? "");
  return (
    <li className="flex flex-col gap-3 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
      <fieldset className="space-y-1">
        {variants.map((v) => (
          <label key={v.id} className="flex items-center gap-2 text-sm">
            <input
              type="radio"
              name={`keep-${variants.map((x) => x.id).join("-")}`}
              checked={keepId === v.id}
              onChange={() => setKeepId(v.id)}
            />
            {v.name}
          </label>
        ))}
      </fieldset>
      <Button
        variant="secondary"
        disabled={busy || !keepId || variants.length < 2}
        onClick={() => onMerge(keepId, variants.map((v) => v.id).filter((id) => id !== keepId))}
      >
        Merge into selected
      </Button>
    </li>
  );
}
