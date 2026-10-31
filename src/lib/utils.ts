import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
export const cn = (...inputs: ClassValue[]) => twMerge(clsx(inputs));
export const active = (status: string) => status === 'downloading' || status === 'converting';
export const retryable = (status: string) => ['failed', 'cancelled', 'interrupted'].includes(status);
export function errorMessage(error: unknown): string { if (typeof error === 'object' && error && 'message' in error) return String(error.message); return String(error); }
export function duration(seconds: number | null): string { if (seconds === null || !Number.isFinite(seconds)) return '—'; const n = Math.max(0, Math.round(seconds)); return n >= 3600 ? `${Math.floor(n / 3600)}:${String(Math.floor(n % 3600 / 60)).padStart(2, '0')}:${String(n % 60).padStart(2, '0')}` : `${Math.floor(n / 60)}:${String(n % 60).padStart(2, '0')}`; }
export function bytes(n: number | null): string { if (n === null || !Number.isFinite(n)) return '—'; if (n < 1024) return `${n} B`; const exp = Math.min(3, Math.floor(Math.log(n) / Math.log(1024))); return `${(n / 1024 ** exp).toFixed(1)} ${['B', 'KB', 'MB', 'GB'][exp]}`; }
export function validUrl(input: string): boolean { try { const url = new URL(input); return ['https:', 'http:'].includes(url.protocol) && !!url.hostname && !url.username && !url.password; } catch { return false; } }
export function sourceName(input: string): string { try { const host = new URL(input).hostname.replace(/^www\./, ''); if (host === 'youtu.be' || host === 'youtube.com' || host.endsWith('.youtube.com')) return 'YouTube'; if (host === 'soundcloud.com') return 'SoundCloud'; if (host === 'vimeo.com') return 'Vimeo'; return host; } catch { return 'Website'; } }
