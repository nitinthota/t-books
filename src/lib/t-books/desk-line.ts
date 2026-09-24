/** Human line for the last AMOT decision. Never raw desk words. */

export type DeskBits = {
  deskIntent?: string;
  deskDecision?: string;
  deskAt?: string;
  isDirty?: boolean;
};

export function deskLine(row: DeskBits | null | undefined): string | undefined {
  const d = (row?.deskDecision ?? "").trim().toLowerCase();
  if (d === "park") return "On this PC. Not sent yet.";
  if (d === "refuse") return "Not sent. Someone else changed this. Reload and submit again.";
  if (d === "post") return "Sent.";
  if (d === "stop_refresh") return "Refresh paused. Unsent rows stay on this PC.";
  if (d === "discard_local") return "This PC now matches the company copy.";
  if (row?.isDirty) return "On this PC. Not sent yet.";
  return undefined;
}

export function deskMark(row: DeskBits | null | undefined): string | undefined {
  const d = (row?.deskDecision ?? "").trim().toLowerCase();
  if (d === "park" || d === "refuse") return d === "refuse" ? "Not sent" : "On this PC";
  if (row?.isDirty) return "On this PC";
  return undefined;
}
