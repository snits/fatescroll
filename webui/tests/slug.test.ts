// ABOUTME: Tests for collectionSlug: turns a collection name into a
// ABOUTME: filesystem/zip-safe slug for the export filename and root folder.

import { describe, expect, it } from 'vitest';
import { collectionSlug } from '../src/logic/slug';

describe('collectionSlug', () => {
  it('lowercases and hyphenates non-alphanumeric runs', () => {
    expect(collectionSlug('Kal-Arath Collection!')).toBe('kal-arath-collection');
  });

  it('falls back to "collection" for an empty name', () => {
    expect(collectionSlug('')).toBe('collection');
  });

  it('falls back to "collection" for a name of only symbols', () => {
    expect(collectionSlug('!!!')).toBe('collection');
  });

  it('trims leading and trailing dashes', () => {
    expect(collectionSlug('--Border Marches--')).toBe('border-marches');
  });
});
