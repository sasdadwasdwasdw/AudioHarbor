export type AudioFormat = 'mp3' | 'm4a' | 'flac' | 'wav';
export type Status = 'queued' | 'downloading' | 'converting' | 'completed' | 'failed' | 'cancelled' | 'interrupted';
export interface Settings { outputDir: string; format: AudioFormat; quality: number; concurrency: number; theme: 'system' | 'dark' | 'light'; language: 'en' | 'th'; autoUpdate: boolean }
export interface Media { id: string; title: string; url: string; uploader: string | null; duration: number | null; thumbnail: string | null; source: string; abr: number | null }
export interface Metadata { title: string; isPlaylist: boolean; items: Media[]; total: number; truncated: boolean }
export interface Progress { percent: number | null; downloaded: number | null; total: number | null; speed: number | null; eta: number | null }
export interface Job { id: string; media: Media; status: Status; format: AudioFormat; quality: number; outputDir: string; outputFile: string | null; createdAt: string; updatedAt: string; progress: Progress; error: string | null }
export interface Snapshot { jobs: Job[]; settings: Settings }
export interface Diagnostic { name: string; version: string; available: boolean }
export interface UpdateInfo { version: string; notes: string }
export interface UpdateProgress { downloaded: number; total: number | null; phase: 'downloading' | 'installing' }
export type Page = 'downloader' | 'queue' | 'history' | 'settings' | 'about';
