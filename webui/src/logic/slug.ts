// ABOUTME: Turns a collection name into a filesystem/zip-safe slug used as
// ABOUTME: the exported zip's filename and its root directory inside the archive.

export const collectionSlug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'collection';
