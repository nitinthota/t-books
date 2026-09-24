/** Board ask: tidy English and party/job names from this PC. */

import { listProjects, listVendors, searchOffice } from "./office";
import type { NavId, PurchasePo, SalesPo, SearchHit, VoucherListRow } from "./types";
import { loadVoucherList } from "./vouchers";

const SPELL: Record<string, string> = {
  vouchr: "voucher",
  voucherss: "vouchers",
  voucer: "voucher",
  purshase: "purchase",
  purchas: "purchase",
  purchace: "purchase",
  vendr: "vendor",
  vendoe: "vendor",
  vendour: "vendor",
  projct: "project",
  projet: "project",
  enginering: "engineering",
  enginerring: "engineering",
  recieve: "receive",
  recieved: "received",
  reciept: "receipt",
  payed: "paid",
  payement: "payment",
  paymnet: "payment",
  ammount: "amount",
  ballance: "balance",
  oustanding: "outstanding",
  outsatnding: "outstanding",
  inventry: "inventory",
  inventary: "inventory",
  logisitcs: "logistics",
  logisitics: "logistics",
  tripps: "trips",
  howmuch: "how much",
  wht: "what",
  whch: "which",
  shw: "show",
  serach: "search",
  seach: "search",
  finde: "find",
};

const FILLER = new Set([
  "please",
  "can",
  "you",
  "tell",
  "me",
  "the",
  "a",
  "an",
  "of",
  "for",
  "to",
  "in",
  "on",
  "at",
  "we",
  "i",
  "did",
  "do",
  "does",
  "our",
  "my",
  "this",
  "that",
  "from",
  "with",
  "how",
  "much",
  "show",
  "find",
  "search",
  "paid",
  "pay",
  "voucher",
  "vouchers",
  "purchase",
  "vendor",
  "project",
  "job",
  "sales",
]);

export type BoardAsk = {
  raw: string;
  cleaned: string;
  changed: boolean;
};

export type BoardHit = {
  nav: NavId;
  key: string;
  title: string;
  subtitle: string;
};

function distance(a: string, b: string): number {
  const s = a.toLowerCase();
  const t = b.toLowerCase();
  const n = s.length;
  const m = t.length;
  if (n === 0) return m;
  if (m === 0) return n;
  const prev = new Array<number>(m + 1);
  const cur = new Array<number>(m + 1);
  for (let j = 0; j <= m; j++) prev[j] = j;
  for (let i = 1; i <= n; i++) {
    cur[0] = i;
    for (let j = 1; j <= m; j++) {
      const cost = s[i - 1] === t[j - 1] ? 0 : 1;
      cur[j] = Math.min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + cost);
    }
    for (let j = 0; j <= m; j++) prev[j] = cur[j];
  }
  return prev[m];
}

function closeEnough(typed: string, name: string): boolean {
  const a = typed.trim().toLowerCase();
  const b = name.trim().toLowerCase();
  if (!a || !b) return false;
  if (a === b || b.includes(a) || a.includes(b)) return a.length >= 3 || b.includes(a);
  const d = distance(a, b);
  const cap = b.length <= 8 ? 2 : b.length <= 16 ? 3 : 4;
  return d <= cap && d / Math.max(b.length, 1) <= 0.34;
}

function bestName(typed: string, names: string[]): string | null {
  let win: string | null = null;
  let best = 99;
  for (const name of names) {
    if (!closeEnough(typed, name)) continue;
    const d = distance(typed, name);
    if (d < best) {
      best = d;
      win = name;
    }
  }
  return win;
}

function spellWords(raw: string): string {
  return raw
    .trim()
    .replace(/\s+/g, " ")
    .split(" ")
    .map((w) => {
      const bare = w.toLowerCase().replace(/[^a-z0-9@.-]/g, "");
      const fix = SPELL[bare];
      if (!fix) return w;
      if (fix.includes(" ")) return fix;
      if (w[0] && w[0] === w[0].toUpperCase()) {
        return fix.charAt(0).toUpperCase() + fix.slice(1);
      }
      return fix;
    })
    .join(" ")
    .replace(/\s+/g, " ")
    .trim();
}

export function tidyAsk(raw: string, names: string[] = []): BoardAsk {
  const trimmed = raw.trim().replace(/\s+/g, " ");
  if (!trimmed) return { raw, cleaned: "", changed: false };
  let cleaned = spellWords(trimmed);
  const words = cleaned.split(" ");
  for (let len = Math.min(4, words.length); len >= 1; len--) {
    for (let i = 0; i + len <= words.length; i++) {
      const slice = words.slice(i, i + len).join(" ");
      if (len === 1 && FILLER.has(slice.toLowerCase())) continue;
      if (len === 1 && /^\d+$/.test(slice)) continue;
      const hit = bestName(slice, names);
      if (!hit) continue;
      words.splice(i, len, hit);
      cleaned = words.join(" ");
      i += 0;
      break;
    }
  }
  cleaned = cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
  return { raw, cleaned, changed: cleaned.toLowerCase() !== trimmed.toLowerCase() };
}

function needles(cleaned: string): string[] {
  return cleaned
    .toLowerCase()
    .split(/[^a-z0-9@.-]+/)
    .filter((w) => w.length > 1 && !FILLER.has(w));
}

function navOf(kind: string): NavId {
  if (kind === "vendor") return "vendors";
  if (kind === "project") return "projects";
  if (kind === "inventory") return "inventory";
  if (kind === "logistics") return "logistics";
  if (kind === "purchase" || kind === "payment") return "purchase";
  if (kind === "sales" || kind === "sales_po") return "sales";
  return "vouchers";
}

function matchHay(hay: string, pins: string[]): boolean {
  const h = hay.toLowerCase();
  return pins.some((p) => p.length >= 2 && h.includes(p));
}

export async function runBoardSearch(
  raw: string,
  bills: PurchasePo[],
  orders: SalesPo[],
): Promise<{ ask: BoardAsk; hits: BoardHit[] }> {
  let names: string[] = [];
  try {
    const [vendors, projects] = await Promise.all([listVendors(), listProjects()]);
    names = [
      ...vendors.map((v) => v.vendor).filter(Boolean),
      ...projects.filter(Boolean),
    ];
  } catch {
    /* still tidy English */
  }
  const ask = tidyAsk(raw, names);
  if (!ask.cleaned) return { ask, hits: [] };
  const pins = needles(ask.cleaned);
  const hits: BoardHit[] = [];
  const seen = new Set<string>();
  const push = (hit: BoardHit) => {
    const id = `${hit.nav}:${hit.key}`;
    if (seen.has(id)) return;
    seen.add(id);
    hits.push(hit);
  };

  const voucherNo = pins.find((p) => /^\d{1,6}$/.test(p));
  const po = pins.find((p) => /^pur-\d+/i.test(p) || /^sal-\d+/i.test(p));
  const pay = pins.find((p) => /^pay-\d+/i.test(p));

  try {
    const vouchers: VoucherListRow[] = await loadVoucherList();
    for (const row of vouchers) {
      const hay = `${row.voucherNumber} ${row.vendor} ${row.project} ${row.taxInvoice} ${row.status}`;
      if (
        (voucherNo && String(row.voucherNumber) === voucherNo) ||
        matchHay(hay, pins.filter((p) => !/^\d+$/.test(p) || p === voucherNo))
      ) {
        push({
          nav: "vouchers",
          key: String(row.voucherNumber),
          title: `Voucher ${row.voucherNumber} · ${row.vendor || "no vendor"}`,
          subtitle: row.project || row.status || "",
        });
      }
    }
  } catch {
    /* register may be empty */
  }

  for (const row of bills) {
    const hay = `${row.poNumber} ${row.vendor} ${row.project}`;
    if ((po && row.poNumber.toLowerCase() === po) || matchHay(hay, pins)) {
      push({
        nav: "purchase",
        key: row.poNumber,
        title: `${row.poNumber} · ${row.vendor}`,
        subtitle: row.project || "",
      });
    }
  }
  for (const row of orders) {
    const hay = `${row.poNumber} ${row.client} ${row.project}`;
    if (matchHay(hay, pins)) {
      push({
        nav: "sales",
        key: `${row.poNumber}@${row.project}`,
        title: `${row.poNumber} · ${row.client || row.project}`,
        subtitle: row.project || "",
      });
    }
  }
  if (pay) {
    push({
      nav: "purchase",
      key: pay.toUpperCase(),
      title: pay.toUpperCase(),
      subtitle: "Purchase payment",
    });
  }

  try {
    const office: SearchHit[] = await searchOffice(ask.cleaned);
    for (const hit of office) {
      push({
        nav: navOf(hit.kind),
        key: hit.id,
        title: hit.title,
        subtitle: hit.subtitle || hit.kind,
      });
    }
  } catch {
    /* office search optional */
  }

  return { ask, hits: hits.slice(0, 20) };
}
