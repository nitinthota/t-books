/** Local name matching for vendors and jobs. No network. */

const DROP = new Set([
  "pvt",
  "ltd",
  "limited",
  "llp",
  "llc",
  "inc",
  "co",
  "company",
  "corp",
  "and",
  "the",
  "of",
]);

const SHORT: Record<string, string> = {
  engg: "engineering",
  engr: "engineering",
  eng: "engineering",
  hyd: "hydraulics",
  hydr: "hydraulics",
  const: "construction",
  constr: "construction",
  mfg: "manufacturing",
  manuf: "manufacturing",
  tech: "technology",
  techno: "technology",
  soln: "solutions",
  solns: "solutions",
  intl: "international",
  ind: "industries",
  indl: "industrial",
  elec: "electrical",
  elect: "electrical",
  mech: "mechanical",
  auto: "automation",
  pvt: "",
  ltd: "",
};

export type NameHit = {
  id: string;
  name: string;
};

export type NameGroup = {
  key: string;
  why: string;
  score: number;
  variants: NameHit[];
};

function tokens(name: string): string[] {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim()
    .split(/\s+/)
    .map((t) => SHORT[t] ?? t)
    .filter((t) => t && !DROP.has(t));
}

export function foldName(name: string): string {
  return tokens(name).sort().join(" ");
}

function levenshtein(a: string, b: string): number {
  if (a === b) return 0;
  if (!a.length) return b.length;
  if (!b.length) return a.length;
  const row = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    let prev = i - 1;
    row[0] = i;
    for (let j = 1; j <= b.length; j++) {
      const cur = row[j];
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      row[j] = Math.min(row[j] + 1, row[j - 1] + 1, prev + cost);
      prev = cur;
    }
  }
  return row[b.length];
}

function tokenScore(a: string[], b: string[]): number {
  if (!a.length || !b.length) return 0;
  const A = new Set(a);
  const B = new Set(b);
  let hit = 0;
  for (const t of A) if (B.has(t)) hit += 1;
  return (200 * hit) / (A.size + B.size);
}

export function nameScore(left: string, right: string): { score: number; why: string } {
  const ta = tokens(left);
  const tb = tokens(right);
  const fa = ta.slice().sort().join(" ");
  const fb = tb.slice().sort().join(" ");
  if (!fa || !fb) return { score: 0, why: "" };
  if (fa === fb) return { score: 100, why: "Same words, different spelling or short form" };

  const overlap = tokenScore(ta, tb);
  const dist = levenshtein(fa, fb);
  const maxLen = Math.max(fa.length, fb.length);
  const edit = maxLen ? Math.round(100 * (1 - dist / maxLen)) : 0;
  const score = Math.max(overlap, edit);

  if (overlap >= 60 && edit >= 50) return { score, why: "Same core words" };
  if (edit >= 78) return { score, why: "Close spelling" };
  if (overlap >= 50) return { score, why: "Shared words" };
  return { score, why: "" };
}

export function groupSimilarNames(rows: NameHit[], floor = 78): NameGroup[] {
  const unique = new Map<string, NameHit>();
  for (const row of rows) {
    const name = row.name.trim();
    if (!name) continue;
    const id = (row.id || name).trim();
    unique.set(id.toLowerCase(), { id, name });
  }
  const list = [...unique.values()];
  const used = new Set<string>();
  const groups: NameGroup[] = [];

  for (let i = 0; i < list.length; i++) {
    if (used.has(list[i].id)) continue;
    const bucket: NameHit[] = [list[i]];
    let why = "";
    let best = 0;
    for (let j = i + 1; j < list.length; j++) {
      if (used.has(list[j].id)) continue;
      const hit = nameScore(list[i].name, list[j].name);
      if (hit.score < floor || !hit.why) continue;
      bucket.push(list[j]);
      if (hit.score > best) {
        best = hit.score;
        why = hit.why;
      }
    }
    const names = new Set(bucket.map((v) => v.name.toLowerCase()));
    if (names.size < 2) continue;
    for (const row of bucket) used.add(row.id);
    groups.push({
      key: foldName(list[i].name) || list[i].name.toLowerCase(),
      why: why || "Similar names",
      score: best || 100,
      variants: bucket,
    });
  }
  return groups.sort((a, b) => b.score - a.score);
}
