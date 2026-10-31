import { invoke, isTauri } from '@tauri-apps/api/core';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import type { AudioFormat, Diagnostic, Job, Metadata, Settings, Snapshot, UpdateInfo } from './types';
export const desktop = isTauri();
function call<T>(command: string, args?: Record<string, unknown>): Promise<T> { if (!desktop) return Promise.reject(new Error('Open AudioHarbor in the desktop app to use this feature.')); return invoke<T>(command, args); }
export const api = {
  snapshot: () => call<Snapshot>('snapshot'),
  inspect: (url: string) => call<Metadata>('inspect_url', { url }),
  enqueue: (urls: string[], format: AudioFormat, quality: number, playlistConfirmed: boolean) => call<Job[]>('enqueue', { urls, format, quality, playlistConfirmed }),
  cancel: (id: string) => call<void>('cancel_job', { id }),
  retry: (id: string) => call<void>('retry_job', { id }),
  clear: () => call<void>('clear_history'),
  saveSettings: (settings: Settings) => call<Settings>('save_settings', { settings }),
  chooseFolder: () => call<string | null>('choose_folder'),
  open: (id: string, folder = false) => call<void>('open_output', { id, folder }),
  diagnostics: () => call<Diagnostic[]>('diagnostics'),
  info: () => call<{ version: string; updaterConfigured: boolean }>('app_info'),
  checkUpdates: () => call<UpdateInfo | null>('check_updates'),
  installUpdate: () => call<void>('install_update'),
  clipboard: () => desktop ? readText() : navigator.clipboard.readText(),
};
