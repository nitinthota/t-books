/** T Books roles stay owner / admin / operator. Loopbook viewer → fail-closed mutate. */

export function normalizeRole(role: string): string {
  return role.trim().toLowerCase();
}

export function canMutate(role: string): boolean {
  return ["owner", "admin", "operator"].includes(normalizeRole(role));
}

export function canWrite(role: string): boolean {
  return canMutate(role);
}

export function canRefreshRole(role: string): boolean {
  return ["owner", "admin"].includes(normalizeRole(role));
}

export function canOpenAccessRole(role: string): boolean {
  return normalizeRole(role) === "owner";
}

export function requireWrite(role: string): void {
  if (!canWrite(role)) {
    throw new Error("You cannot change the books with this role.");
  }
}

export function requireAdmin(role: string): void {
  if (!["owner", "admin"].includes(normalizeRole(role))) {
    throw new Error("You cannot refresh or administer with this role.");
  }
}
