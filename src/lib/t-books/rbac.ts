import { OWNER_EMAIL } from "./constants";
import { ROLES, type Role } from "./types";

export function normalizeEmail(email: string): string {
  return email.trim().toLowerCase();
}

export function isHardcodedOwner(email: string): boolean {
  return normalizeEmail(email) === OWNER_EMAIL;
}

export function isRole(value: string): value is Role {
  return (ROLES as readonly string[]).includes(value);
}

export function canEditBooks(role: Role): boolean {
  return role === "owner" || role === "admin" || role === "operator";
}

export const canMutate = canEditBooks;

export function canRefresh(role: Role): boolean {
  return role === "owner" || role === "admin";
}

export function canOpenAccess(role: Role): boolean {
  return role === "owner";
}

export function isValidEmailShape(email: string): boolean {
  const value = normalizeEmail(email);
  const at = value.indexOf("@");
  if (at <= 0) return false;
  const domain = value.slice(at + 1);
  return domain.includes(".") && domain.length >= 3 && !value.includes(" ");
}
