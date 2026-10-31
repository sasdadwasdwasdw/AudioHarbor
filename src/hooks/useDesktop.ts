import { useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { api, desktop } from '../lib/api';
import { useApp } from '../store/app';
import type { Job, UpdateProgress } from '../lib/types';
export function useDesktop() {
  useEffect(() => {
    if (!desktop) return;
    let disposed = false; const unlisten: UnlistenFn[] = []; let hydrated = false; const buffered: Job[] = [];
    const subscribe = async <T,>(event: string, handler: (payload: T) => void) => { const stop = await listen<T>(event, ({ payload }) => { if (!disposed) handler(payload); }); if (disposed) stop(); else unlisten.push(stop); };
    void (async () => { try {
      await subscribe<Job>('job-changed', (job) => hydrated ? useApp.getState().upsert(job) : buffered.push(job));
      await subscribe<unknown>('app-error', (error) => useApp.getState().report(error));
      await subscribe('history-cleared', () => { void useApp.getState().refresh(); });
      await subscribe<UpdateProgress>('update-progress', (updateProgress) => useApp.setState({ updateProgress }));
      const snapshot = await api.snapshot(); const info = await api.info(); if (disposed) return;
      useApp.getState().hydrate(snapshot); buffered.forEach((job) => useApp.getState().upsert(job)); hydrated = true; useApp.setState(info);
      if (snapshot.settings.autoUpdate && info.updaterConfigured) void useApp.getState().checkUpdates();
    } catch (error) { if (!disposed) useApp.getState().report(error); } })();
    return () => { disposed = true; unlisten.forEach((stop) => stop()); };
  }, []);
  const theme = useApp((s) => s.settings?.theme ?? 'system');
  const language = useApp((s) => s.settings?.language ?? 'en');
  useEffect(() => { const media = matchMedia('(prefers-color-scheme: dark)'); const apply = () => document.documentElement.classList.toggle('dark', theme === 'dark' || (theme === 'system' && media.matches)); apply(); media.addEventListener('change', apply); return () => media.removeEventListener('change', apply); }, [theme]);
  useEffect(() => { document.documentElement.lang = language; }, [language]);
}
