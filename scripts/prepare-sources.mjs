import { createHash } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
// Distributor-supplied complete corresponding FFmpeg + dependency sources and build
// recipes are mandatory for public release. This gate must not be bypassed.
const url = process.env.FFMPEG_SOURCE_ARCHIVE_URL; const expected = process.env.FFMPEG_SOURCE_ARCHIVE_SHA256;
if (!url?.startsWith('https://') || !/^[a-f0-9]{64}$/i.test(expected ?? '')) throw new Error('Provide FFMPEG_SOURCE_ARCHIVE_URL and FFMPEG_SOURCE_ARCHIVE_SHA256 for the exact pinned LGPL build and its linked dependencies. See THIRD_PARTY_NOTICES.md.');
const response = await fetch(url, { signal: AbortSignal.timeout(300_000) }); if (!response.ok) throw new Error(`Source archive HTTP ${response.status}`);
const data = Buffer.from(await response.arrayBuffer()); if (createHash('sha256').update(data).digest('hex') !== expected.toLowerCase()) throw new Error('Source archive SHA256 mismatch.');
await mkdir(new URL('../.cache/release-sources/', import.meta.url), { recursive: true });
await writeFile(new URL('../.cache/release-sources/ffmpeg-corresponding-source.tar.xz', import.meta.url), data);
console.log('Corresponding source archive verified for publishing alongside the installer.');
