// ABOUTME: Browser file-save helper: wraps a Blob in an object URL and clicks
// ABOUTME: a temporary anchor to trigger the download dialog.

export function triggerDownload(filename: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  // Synchronous revoke is safe: the browser resolves the blob reference
  // during the click, before the URL is invalidated.
  URL.revokeObjectURL(url);
}
