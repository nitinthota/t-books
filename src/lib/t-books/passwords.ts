import {
  ARGON2_HASH_LENGTH,
  ARGON2_ITERATIONS,
  ARGON2_MEMORY_KIB,
  ARGON2_PARALLELISM,
  MIN_PASSWORD_LENGTH,
} from "./constants";

export function validateNewPassword(password: string, email?: string): string {
  const value = password.trim();
  if (value.length < MIN_PASSWORD_LENGTH) {
    throw new Error(`Password must be at least ${MIN_PASSWORD_LENGTH} characters.`);
  }
  if (email && value.toLowerCase() === email.trim().toLowerCase()) {
    throw new Error("Password cannot be the same as the email.");
  }
  return value;
}

/** Argon2id PHC string. Never log this value. */
export async function hashPassword(plain: string): Promise<string> {
  const { argon2id } = await import("hash-wasm");
  const salt = crypto.getRandomValues(new Uint8Array(16));
  return argon2id({
    password: plain,
    salt,
    parallelism: ARGON2_PARALLELISM,
    iterations: ARGON2_ITERATIONS,
    memorySize: ARGON2_MEMORY_KIB,
    hashLength: ARGON2_HASH_LENGTH,
    outputType: "encoded",
  });
}

export async function verifyPassword(
  plain: string,
  stored: string | null | undefined,
): Promise<boolean> {
  if (!stored) return false;
  try {
    const { argon2Verify } = await import("hash-wasm");
    return await argon2Verify({ password: plain, hash: stored });
  } catch {
    return false;
  }
}
