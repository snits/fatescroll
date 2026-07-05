// ABOUTME: Cosmetic namespace-shape check shared by the manifest and table
// ABOUTME: editors — a UI affordance only; the engine is the authority.

const NAMESPACE_PATTERN = /^[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)*$/;

export function isValidNamespace(namespace: string): boolean {
  return NAMESPACE_PATTERN.test(namespace);
}
