import { accessDenialMessage, sessionRole } from "./access";
import { all, flushLocalBooks, run } from "./db";
import { hashPassword, validateNewPassword, verifyPassword } from "./passwords";
import { isValidEmailShape, normalizeEmail } from "./rbac";
import type { AuthInspect, AuthResult, Session } from "./types";

type UserRow = {
  email: string;
  password_hash: string;
};

function getUser(email: string): UserRow | null {
  const rows = all<UserRow>(
    `SELECT email, password_hash FROM users_local WHERE email = ? COLLATE NOCASE LIMIT 1`,
    [email],
  );
  return rows[0] ?? null;
}

function sessionFor(email: string): Session {
  return { email, role: sessionRole(email) };
}

export function inspectEmail(rawEmail: string): AuthInspect {
  const email = normalizeEmail(rawEmail);
  if (!isValidEmailShape(email)) {
    return { kind: "denied", message: "Enter a valid email." };
  }
  const denied = accessDenialMessage(email);
  if (denied) return { kind: "denied", message: denied };
  return getUser(email) ? { kind: "password", email } : { kind: "set-password", email };
}

export async function signIn(rawEmail: string, password: string): Promise<AuthResult> {
  const inspected = inspectEmail(rawEmail);
  if (inspected.kind === "denied") return { ok: false, message: inspected.message };
  if (inspected.kind === "set-password") {
    return { ok: false, message: "Set a password for this PC first." };
  }
  const user = getUser(inspected.email);
  if (!user) return { ok: false, message: "Could not open that account on this PC." };
  const match = await verifyPassword(password, user.password_hash);
  if (!match) return { ok: false, message: "Invalid credentials." };
  return { ok: true, session: sessionFor(user.email) };
}

export async function setFirstPassword(
  rawEmail: string,
  password: string,
  confirm: string,
): Promise<AuthResult> {
  const inspected = inspectEmail(rawEmail);
  if (inspected.kind === "denied") return { ok: false, message: inspected.message };
  if (inspected.kind !== "set-password") {
    return { ok: false, message: "This PC already has a password for that email." };
  }
  if (password !== confirm) {
    return { ok: false, message: "Those passwords do not match." };
  }
  let next: string;
  try {
    next = validateNewPassword(password, inspected.email);
  } catch (err) {
    return { ok: false, message: err instanceof Error ? err.message : "Password is not valid." };
  }
  const hash = await hashPassword(next);
  run(
    `INSERT INTO users_local (email, password_hash, created_at) VALUES (?, ?, datetime('now'))`,
    [inspected.email, hash],
  );
  await flushLocalBooks();
  return { ok: true, session: sessionFor(inspected.email) };
}
