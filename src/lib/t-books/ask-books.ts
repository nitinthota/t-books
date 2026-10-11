import { invokeCommand, isTauriRuntime } from "./platform";
import { loadVoucherList } from "./vouchers";
import { listPurchasePo } from "./office";

export type AskPeriod = { year?: number; month?: number; label: string };
export type AskIntent = {
  raw: string;
  pendingLike: boolean;
  partial: boolean;
  full: boolean;
  advance: boolean;
  missingTax: boolean;
  scope: "vouchers" | "purchase" | "both";
  periods: AskPeriod[];
};
export type AskBucket = {
  label: string;
  voucherCount: number;
  voucherRemaining: number;
  purchaseCount: number;
  purchaseValue: number;
  sampleVoucherNos: number[];
};
export type AskResult = { answer: string; buckets: AskBucket[]; intent: AskIntent };

const MONTHS: Record<string, number> = {
  jan: 1, january: 1, feb: 2, february: 2, mar: 3, march: 3, apr: 4, april: 4,
  may: 5, jun: 6, june: 6, jul: 7, july: 7, aug: 8, august: 8, sep: 9, september: 9,
  oct: 10, october: 10, nov: 11, november: 11, dec: 12, december: 12,
};

export function parseAskIntent(question: string): AskIntent {
  const raw = question.trim();
  const q = raw.toLowerCase();
  const pendingLike = /pending|still to pay|\bdue\b|unpaid|outstanding/.test(q);
  const partial = /partial/.test(q);
  const full = /fully paid|paid in full/.test(q);
  const advance = /advance/.test(q);
  const missingTax = /missing tax|no tax inv/.test(q);
  let scope: AskIntent["scope"] = "both";
  if (/purchase/.test(q) && !/voucher/.test(q)) scope = "purchase";
  else if (/voucher/.test(q) && !/purchase/.test(q)) scope = "vouchers";
  const periods: AskPeriod[] = [];
  for (const [name, month] of Object.entries(MONTHS)) {
    const m = q.match(new RegExp(`\\b${name}\\s+(20\\d{2})\\b`));
    if (m) periods.push({ year: +m[1], month, label: `${name} ${m[1]}` });
  }
  for (const m of q.matchAll(/\b(20\d{2})\b/g)) {
    const y = +m[1];
    if (!periods.some((p) => p.year === y)) periods.push({ year: y, label: String(y) });
  }
  return {
    raw,
    pendingLike: pendingLike || (!partial && !full && !advance && !missingTax),
    partial, full: full && !pendingLike, advance, missingTax, scope,
    periods: periods.length ? periods : [{ label: "all dates on this PC" }],
  };
}

export function parseBookDate(raw: string): { year?: number; month?: number } {
  const s = (raw || "").trim();
  let m = s.match(/^(20\d{2})-(\d{1,2})/);
  if (m) return { year: +m[1], month: +m[2] };
  m = s.match(/^(\d{1,2})[.\/](\d{1,2})[.\/](20\d{2})/);
  if (m) return { year: +m[3], month: +m[2] };
  m = s.match(/(20\d{2})/);
  return m ? { year: +m[1] } : {};
}

function statusMatches(status: string, remaining: number, intent: AskIntent): boolean {
  const st = (status || "").toLowerCase();
  if (intent.missingTax) return st.includes("missing");
  if (intent.advance) return st.includes("advance");
  if (intent.partial) return st.includes("partial");
  if (intent.full) return st.includes("full") && !st.includes("partial");
  if (intent.pendingLike) return /pending|partial|missing/.test(st) || remaining > 0.004;
  return true;
}

function inPeriod(y: number | undefined, m: number | undefined, p: AskPeriod): boolean {
  if (p.year && y !== p.year) return false;
  if (p.month && m !== p.month) return false;
  return true;
}

function money(n: number) {
  return new Intl.NumberFormat("en-IN", { style: "currency", currency: "INR", maximumFractionDigits: 0 }).format(n || 0);
}

export async function askBooksLocal(question: string): Promise<AskResult> {
  const intent = parseAskIntent(question);
  const [vouchers, purchase] = await Promise.all([loadVoucherList(), listPurchasePo()]);
  const buckets: AskBucket[] = [];
  for (const period of intent.periods) {
    let voucherCount = 0, voucherRemaining = 0, purchaseCount = 0, purchaseValue = 0;
    const sampleVoucherNos: number[] = [];
    if (intent.scope !== "purchase") {
      for (const v of vouchers as any[]) {
        const d = parseBookDate(v.voucherDate || v.date || "");
        if ((period.year || period.month) && !d.year) continue;
        if ((period.year || period.month) && !inPeriod(d.year, d.month, period)) continue;
        if (!statusMatches(v.status || "", v.remaining || 0, intent)) continue;
        voucherCount++;
        voucherRemaining += Number(v.remaining) || 0;
        if (sampleVoucherNos.length < 8) sampleVoucherNos.push(v.voucherNumber);
      }
    }
    if (intent.scope !== "vouchers") {
      for (const p of purchase) {
        const d = parseBookDate(p.createdAt || "");
        if ((period.year || period.month) && !d.year) continue;
        if ((period.year || period.month) && !inPeriod(d.year, d.month, period)) continue;
        purchaseCount++;
        purchaseValue += Number(p.totalValue) || 0;
      }
    }
    buckets.push({ label: period.label, voucherCount, voucherRemaining, purchaseCount, purchaseValue, sampleVoucherNos });
  }
  const focus = intent.pendingLike ? "still to pay / pending-like" : "matching";
  const lines = [`On this PC (${focus}):`];
  for (const b of buckets) {
    lines.push(`• ${b.label}: ${b.voucherCount} vouchers · still to pay ${money(b.voucherRemaining)}; ${b.purchaseCount} purchase bills.`);
    if (b.sampleVoucherNos.length) lines.push(`  Examples: ${b.sampleVoucherNos.map((n) => `V-${n}`).join(", ")}.`);
  }
  lines.push("Figures are from local books only.");
  return { answer: lines.join("\n"), buckets, intent };
}

export async function askBooks(question: string): Promise<AskResult> {
  const q = question.trim();
  if (!q) {
    return { answer: "Ask e.g. How many bills are pending in 2025 and June 2026?", buckets: [], intent: parseAskIntent("") };
  }
  if (isTauriRuntime()) {
    try { return await invokeCommand<AskResult>("ask_books", { question: q }); }
    catch { return askBooksLocal(q); }
  }
  return askBooksLocal(q);
}
