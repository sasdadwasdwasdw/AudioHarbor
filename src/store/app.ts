import { create } from 'zustand';
import { api } from '../lib/api';
import { errorMessage } from '../lib/utils';
import type { Job, Page, Settings, Snapshot, UpdateInfo, UpdateProgress } from '../lib/types';
interface AppStore {
  page: Page; settings: Settings | null; jobs: Job[]; connected: boolean; notice: string | null; error: string | null;
  update: UpdateInfo | null; updateProgress: UpdateProgress | null; checking: boolean; installing: boolean; updateStatus: string | null; updateDismissed: boolean; version: string; updaterConfigured: boolean;
  navigate: (page: Page) => void; hydrate: (snapshot: Snapshot) => void; upsert: (job: Job) => void;
  report: (error: unknown) => void; notify: (message: string) => void; refresh: () => Promise<void>; checkUpdates: () => Promise<void>; installUpdate: () => Promise<void>;
}
export const useApp = create<AppStore>((set) => ({
  page: 'downloader', settings: null, jobs: [], connected: false, notice: null, error: null,
  update: null, updateProgress: null, checking: false, installing: false, updateStatus: null, updateDismissed: false, version: '0.1.0', updaterConfigured: false,
  navigate: (page) => set({ page }), hydrate: (snapshot) => set({ ...snapshot, connected: true }),
  upsert: (job) => set((state) => { const old = state.jobs.find((j) => j.id === job.id); if (old && Date.parse(old.updatedAt) > Date.parse(job.updatedAt)) return {}; return { jobs: old ? state.jobs.map((j) => j.id === job.id ? job : j) : [...state.jobs, job] }; }),
  report: (error) => set({ error: errorMessage(error), notice: null }), notify: (notice) => set({ notice, error: null }),
  refresh: async () => { try { const snapshot = await api.snapshot(); set({ ...snapshot, connected: true }); } catch (error) { set({ error: errorMessage(error) }); } },
  checkUpdates: async () => { set({ checking: true, updateStatus: null }); try { const update = await api.checkUpdates(); set({ update, updateDismissed: false, updateStatus: update ? `Version ${update.version} is available.` : 'You’re up to date.' }); } catch (error) { set({ updateStatus: errorMessage(error) }); } finally { set({ checking: false }); } },
  installUpdate: async () => { set({ installing: true, updateProgress: null, updateStatus: 'Downloading and verifying the signed update…' }); try { await api.installUpdate(); set({ updateStatus: 'Installer launched. AudioHarbor will close during installation.' }); } catch (error) { set({ updateStatus: errorMessage(error), installing: false, updateProgress: null }); } },
}));
