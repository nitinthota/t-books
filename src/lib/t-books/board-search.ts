/** Board ask: tidy English, then look on this PC. No cloud model. */

import { searchOffice } from "./office";
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

export function tidyAsk(raw: string): BoardAsk {
  const trimmed = raw.trim().replace(/\s+/g, " ");
  if (!trimmed) return { raw, cleaned: "", changed: false };
  const words = trimmed.split(" ").map((w) => {
    const bare = w.toLowerCase().replace(/[^a-z0-9@.-]/g, "");
    return SPELL[bare] ?? w;
  });
  let cleaned = words.join(" ").replace(/\s+/g, " ").trim();
  cleaned = cleaned.replace(/\bhowmuch\b/gi, "how much");
  cleaned = cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
  if (!/[.?!]$/.test(cleaned)) {
    /* keep as a look-up, not a forced period */
  }
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
  const ask = tidyAsk(raw);
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
