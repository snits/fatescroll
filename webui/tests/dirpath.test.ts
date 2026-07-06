// ABOUTME: Tests for isValidDirPath: the manifest editor's directory-path
// ABOUTME: shape check that keeps zip entry paths relative and portable.

import { describe, expect, it } from 'vitest';
import { isValidDirPath } from '../src/logic/dirpath';

describe('isValidDirPath', () => {
  it('accepts a single segment', () => {
    expect(isValidDirPath('core')).toBe(true);
  });

  it('accepts nested segments', () => {
    expect(isValidDirPath('core/weather')).toBe(true);
  });

  it('accepts a trailing slash (collectionFiles strips it)', () => {
    expect(isValidDirPath('core/')).toBe(true);
  });

  it('rejects the empty string', () => {
    expect(isValidDirPath('')).toBe(false);
  });

  it('rejects an absolute path', () => {
    expect(isValidDirPath('/abs')).toBe(false);
  });

  it('rejects empty segments', () => {
    expect(isValidDirPath('a//b')).toBe(false);
  });

  it('rejects parent-directory segments', () => {
    expect(isValidDirPath('../up')).toBe(false);
  });

  it('rejects current-directory segments', () => {
    expect(isValidDirPath('a/./b')).toBe(false);
  });
});
