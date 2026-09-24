/** Human line for the last AMOT decision. Never raw desk words. */

export type DeskBits = {
  deskIntent?: string;
  deskDecision?: string;
  deskAt?: string;
  isDirty?: boolean;
};

export function deskLine(row: DeskBits | null | undefined): string | undefined {
  const d = (row?.deskDecision ?? "").trim().toLowerCase();
  if (d === "park") return "Draft. Not posted.";
  if (d === "refuse") return "Not posted. Reload and post again.";
  if (d === "post") return "Posted.";
  if (d === "stop_refresh") return "Refresh paused. Unposted drafts stay here.";
  if (d === "discard_local") return "Matches the company copy.";
  if (row?.isDirty) return "Draft. Not posted.";
  return undefined;
}

export function deskMark(row: DeskBits | null | undefined): string | undefined {
  const d = (row?.deskDecision ?? "").trim().toLowerCase();
  if (d === "park" || d === "refuse") return d === "refuse" ? "Not posted" : "Draft";
  if (row?.isDirty) return "Draft";
  return undefined;
}
