import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { listDuplicates, mergeMaster } from "@/lib/t-books/control";
import { groupSimilarNames, type NameGroup } from "@/lib/t-books/name-match";
import { listProjects, listPurchasePo, listVendors } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { loadVoucherList } from "@/lib/t-books/vouchers";
import { ModuleFrame } from "./module-frame";

export default function DuplicatesScreen({ onBack }: { onBack: () => void }) {
  const [serials, setSerials] = useState<{ voucherNo: string; count: number }[]>([]);
  const [vendors, setVendors] = useState<NameGroup[]>([]);
  const [jobs, setJobs] = useState<NameGroup[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const [report, vendorRows, projects, vouchers, bills] = await Promise.all([
        listDuplicates(),
        listVendors(),
        listProjects(),
        loadVoucherList(),
        listPurchasePo(),
      ]);
      setSerials(report.serials);
      setVendors(
        groupSimilarNames([
          ...vendorRows.map((v) => ({ id: v.vendor, name: v.vendor })),
          ...vouchers.map((v) => ({ id: v.vendor, name: v.vendor })),
          ...bills.map((v) => ({ id: v.vendor, name: v.vendor })),
          ...report.vendors.flatMap((g) => g.variants),
        ]),
      );
      setJobs(
        groupSimilarNames([
          ...projects.map((p) => ({ id: p, name: p })),
          ...vouchers.map((v) => ({ id: v.project, name: v.project })),
          ...bills.map((v) => ({ id: v.project, name: v.project })),
          ...report.projects.flatMap((g) => g.variants),
        ]),
      );
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
      hint="Close spellings and short forms are grouped here. You pick the name to keep. Voucher numbers are never merged."
      onBack={onBack}
      error={error}
    >
      <Section title="Voucher numbers" empty="No duplicate voucher numbers." count={serials.length}>
        {serials.map((s) => (
          <li key={s.voucherNo} className="flex justify-between px-4 py-3">
            <span className="font-mono text-sm">{s.voucherNo}</span>
            <span className="text-sm text-ink-muted">{s.count} times · not merged</span>
          </li>
        ))}
      </Section>
      <MergeGroups
        title="Vendor names"
        empty="No similar vendor names."
        groups={vendors}
        busy={busy}
        onMerge={(keep, absorb) => void onMerge("vendor", keep, absorb)}
      />
      <MergeGroups
        title="Job names"
        empty="No similar job names."
        groups={jobs}
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
  groups: NameGroup[];
  busy: boolean;
  onMerge: (keepId: string, absorbIds: string[]) => void;
}) {
  return (
    <Section title={title} empty={empty} count={groups.length}>
      {groups.map((g) => (
        <MergeRow key={g.key + g.variants.map((v) => v.id).join("-")} group={g} busy={busy} onMerge={onMerge} />
      ))}
    </Section>
  );
}

function MergeRow({
  group,
  busy,
  onMerge,
}: {
  group: NameGroup;
  busy: boolean;
  onMerge: (keepId: string, absorbIds: string[]) => void;
}) {
  const variants = group.variants;
  const [keepId, setKeepId] = useState(variants[0]?.id ?? "");
  return (
    <li className="flex flex-col gap-3 border-t border-line px-4 py-3 sm:flex-row sm:items-start sm:justify-between">
      <fieldset className="space-y-1">
        <p className="text-xs text-ink-subtle">
          {group.why} · {group.score}
        </p>
        {variants.map((v) => (
          <label key={v.id} className="flex items-center gap-2 text-sm">
            <input
              type="radio"
              name={`keep-${group.key}-${variants.map((x) => x.id).join("-")}`}
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
        Keep this name
      </Button>
    </li>
  );
}
