import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  listDuplicates,
  mergeMaster,
  type DuplicateGroup,
  type DuplicatesReport,
} from "@/lib/t-books/control";
import { listProjects, listPurchasePo, listVendors } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { loadVoucherList } from "@/lib/t-books/vouchers";
import { ModuleFrame } from "./module-frame";

const DROP = new Set([
  "pvt",
  "ltd",
  "limited",
  "co",
  "company",
  "engg",
  "engineering",
  "and",
  "the",
]);

function tokens(name: string): string[] {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim()
    .split(/\s+/)
    .filter((t) => t && !DROP.has(t));
}

function fold(name: string): string {
  return tokens(name).sort().join(" ");
}

function close(a: string, b: string): boolean {
  if (!a || !b) return false;
  if (a === b) return true;
  const A = new Set(a.split(" "));
  const B = new Set(b.split(" "));
  let hit = 0;
  for (const t of A) if (B.has(t) && t.length > 2) hit += 1;
  if (hit >= 2) return true;
  if (A.size === 1 && B.has([...A][0])) return true;
  if (B.size === 1 && A.has([...B][0])) return true;
  return false;
}

function groupNames(rows: Array<{ id: string; name: string }>): DuplicateGroup[] {
  const unique = new Map<string, { id: string; name: string }>();
  for (const row of rows) {
    const name = row.name.trim();
    if (!name) continue;
    const id = row.id.trim() || name;
    unique.set(id.toLowerCase(), { id, name });
  }
  const list = [...unique.values()].map((row) => ({ ...row, key: fold(row.name) }));
  const used = new Set<string>();
  const groups: DuplicateGroup[] = [];
  for (let i = 0; i < list.length; i++) {
    if (used.has(list[i].id)) continue;
    const bucket = [list[i]];
    for (let j = i + 1; j < list.length; j++) {
      if (used.has(list[j].id)) continue;
      if (close(list[i].key, list[j].key)) bucket.push(list[j]);
    }
    const names = new Set(bucket.map((v) => v.name.toLowerCase()));
    if (names.size < 2) continue;
    for (const row of bucket) used.add(row.id);
    groups.push({
      key: bucket[0].key || bucket[0].name.toLowerCase(),
      variants: bucket.map(({ id, name }) => ({ id, name })),
    });
  }
  return groups;
}

export default function DuplicatesScreen({ onBack }: { onBack: () => void }) {
  const [data, setData] = useState<DuplicatesReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const [report, vendors, projects, vouchers, bills] = await Promise.all([
        listDuplicates(),
        listVendors(),
        listProjects(),
        loadVoucherList(),
        listPurchasePo(),
      ]);
      const vendorRows = [
        ...vendors.map((v) => ({ id: v.vendor, name: v.vendor })),
        ...vouchers.map((v) => ({ id: v.vendor, name: v.vendor })),
        ...bills.map((v) => ({ id: v.vendor, name: v.vendor })),
        ...report.vendors.flatMap((g) => g.variants),
      ];
      const projectRows = [
        ...projects.map((p) => ({ id: p, name: p })),
        ...vouchers.map((v) => ({ id: v.project, name: v.project })),
        ...bills.map((v) => ({ id: v.project, name: v.project })),
        ...report.projects.flatMap((g) => g.variants),
      ];
      setData({
        serials: report.serials,
        vendors: groupNames(vendorRows),
        projects: groupNames(projectRows),
      });
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
      hint="Similar names are listed. You pick the name to keep. Duplicate voucher numbers are never merged."
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
        empty="No similar vendor names."
        groups={data?.vendors ?? []}
        busy={busy}
        onMerge={(keep, absorb) => void onMerge("vendor", keep, absorb)}
      />
      <MergeGroups
        title="Job names"
        empty="No similar job names."
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
  group: DuplicateGroup;
  busy: boolean;
  onMerge: (keepId: string, absorbIds: string[]) => void;
}) {
  const variants = group.variants;
  const [keepId, setKeepId] = useState(variants[0]?.id ?? "");
  return (
    <li className="flex flex-col gap-3 border-t border-line px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
      <fieldset className="space-y-1">
        <p className="text-xs text-ink-subtle">Similar to “{group.key}”</p>
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
