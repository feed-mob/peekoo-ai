/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_FORCE_UPDATER_DIALOG?: string;
  readonly VITE_FORCE_UPDATER_VERSION?: string;
  readonly VITE_FORCE_UPDATER_NOTES?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
