import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { basename, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unzipSync } from 'fflate';
const root = fileURLToPath(new URL('../', import.meta.url));
const lock = JSON.parse(await readFile(new URL('./engines.lock.json', import.meta.url), 'utf8'));
const cache = resolve(root, '.cache/engines'); const bin = resolve(root, 'src-tauri/binaries'); const resources = resolve(root, 'src-tauri/resources/engines');
await Promise.all([mkdir(cache, { recursive: true }), mkdir(bin, { recursive: true }), mkdir(resources, { recursive: true })]);
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
async function fetchBytes(url) { const response = await fetch(url, { signal: AbortSignal.timeout(180_000) }); if (!response.ok) throw new Error(`Engine download returned HTTP ${response.status}: ${new URL(url).hostname}`); return Buffer.from(await response.arrayBuffer()); }
const manifest = [];
for (const asset of lock.assets) {
  const cached = resolve(cache, `${asset.sha256}-${basename(asset.url)}`); let data;
  try { data = await readFile(cached); } catch { /* Missing cache will be downloaded. */ }
  if (!data || hash(data) !== asset.sha256) { console.log(`Downloading ${asset.name} ${asset.version}…`); data = await fetchBytes(asset.url); }
  if (hash(data) !== asset.sha256) throw new Error(`SHA256 mismatch for ${asset.name}; refusing to use this binary. Review the pinned asset, never disable verification.`);
  await writeFile(cached, data);
  const entries = asset.url.endsWith('.zip') ? unzipSync(data) : { 'yt-dlp.exe': data };
  for (const name of asset.executables) {
    const matches = Object.entries(entries).filter(([key]) => basename(key) === `${name}.exe`);
    if (matches.length !== 1) throw new Error(`Expected exactly one ${name}.exe`);
    const executable = matches[0][1]; await writeFile(resolve(bin, `${name}-${lock.target}.exe`), executable);
    manifest.push({ name, version: asset.version, sha256: hash(executable), origin: asset.url, archiveSha256: asset.sha256 });
  }
  // Keep all upstream license/readme texts and build configuration shipped in archives.
  for (const [key, bytes] of Object.entries(entries)) if (/\.(txt|md|html)$/i.test(key) && !key.endsWith('/')) { await writeFile(resolve(resources, `${asset.name}-${basename(key)}`), bytes); }
}
// Upstream notices at the exact pinned source versions, not moving main branches.
const noticeUrls = {
  'yt-dlp-LICENSE.txt': 'https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.10.26/LICENSE',
  'yt-dlp-THIRD_PARTY_LICENSES.txt': 'https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.10.26/THIRD_PARTY_LICENSES.txt',
  'deno-LICENSE.md': 'https://raw.githubusercontent.com/denoland/deno/v2.9.8/LICENSE.md',
};
for (const [name, url] of Object.entries(noticeUrls)) await writeFile(resolve(resources, name), await fetchBytes(url));
await writeFile(resolve(resources, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
await copyFile(resolve(root, 'scripts/engines.lock.json'), resolve(resources, 'engines.lock.json'));
console.log('All four Windows x64 engines verified and prepared.');
