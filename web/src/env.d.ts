// The File System Access API is not in TypeScript's DOM lib. Only the one call the Save
// flow makes is declared, so an unsupported browser is a `canSaveAs()` check rather than a
// cast at the call site.
interface SaveFilePickerOptions {
  suggestedName?: string;
}

interface Window {
  showSaveFilePicker?: (options?: SaveFilePickerOptions) => Promise<FileSystemFileHandle>;
}
