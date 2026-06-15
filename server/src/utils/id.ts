/**
 * Generate a short user-friendly ID prefix with a UUID suffix.
 * Format: "user-<8 hex chars>" for user IDs.
 */
export function generateUserId(): string {
  const array = new Uint8Array(4);
  crypto.getRandomValues(array);
  const hex = Array.from(array, (b) => b.toString(16).padStart(2, "0")).join("");
  return `user-${hex}`;
}

/**
 * Generate a standard UUID.
 */
export function generateId(): string {
  return crypto.randomUUID();
}
