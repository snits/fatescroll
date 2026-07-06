// ABOUTME: Cosmetic directory-path shape check for the manifest editor — a UI
// ABOUTME: affordance keeping zip entry paths relative and portable; the zip
// ABOUTME: builder is the enforcing guard.

// Trailing slashes are allowed because collectionFiles strips them before
// joining the path with a table stem.
export function isValidDirPath(path: string): boolean {
  if (path.startsWith('/')) return false;
  const trimmed = path.replace(/\/+$/, '');
  if (trimmed === '') return false;
  return trimmed.split('/').every((seg) => seg !== '' && seg !== '.' && seg !== '..');
}
