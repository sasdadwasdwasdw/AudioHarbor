import { readFile } from 'node:fs/promises';
const manifest = JSON.parse(await readFile(process.argv[2], 'utf8'));
const entry = manifest.platforms?.['windows-x86_64'] ?? manifest.platforms?.['windows-x86_64-msi'];
if (!entry?.url?.startsWith('https://github.com/') || !entry.url.endsWith('.msi') || !entry.signature || manifest.version !== process.env.RELEASE_TAG?.slice(1)) throw new Error('Invalid updater manifest: expected correct version, HTTPS MSI URL and Tauri signature.');
console.log('MSI updater manifest shape verified. The installed updater verifies the cryptographic signature.');
