import { readFile, readdir, writeFile, mkdir } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
const root=fileURLToPath(new URL('../',import.meta.url));
const lock=JSON.parse(await readFile(resolve(root,'package-lock.json'),'utf8'));
const notices=['AudioHarbor application dependencies\nGenerated from locked installed dependency sources. Engine notices are shipped separately.\n'];
const missing=[];
const extra=JSON.parse(await readFile(new URL('./extra-notices.lock.json',import.meta.url),'utf8'));
async function collect(name,version,dir,license,source){
  const names=(await readdir(dir)).filter(name=>/^(licen[sc]e|copying|notice)([._-]|$)/i.test(name));
  notices.push(`\n${'='.repeat(72)}\n${name} ${version}\nLicense expression: ${license??'See component source'}\nSource: ${source??''}\n`);
  let count=0;
  for(const name of names){try{const text=await readFile(resolve(dir,name),'utf8');notices.push(`\n--- ${name} ---\n${text}\n`);count++;}catch(error){if(error.code!=='EISDIR')throw error;}}
  if(!count){const fallback=extra.find(item=>item.name===name&&item.version===version);if(fallback){const text=await readFile(resolve(root,fallback.file));if(createHash('sha256').update(text).digest('hex')!==fallback.sha256)throw new Error(`Extra notice hash mismatch for ${name}`);notices.push(`\nUpstream license source: ${fallback.url}\n${text.toString('utf8')}\n`);}else missing.push(`${name} ${version} (${source})`);}
}
for(const [path,pkg] of Object.entries(lock.packages)){
  if(!path||pkg.dev)continue;
  const info=JSON.parse(await readFile(resolve(root,path,'package.json'),'utf8'));
  await collect(info.name,info.version,resolve(root,path),info.license,`https://www.npmjs.com/package/${info.name}/v/${info.version}`);
}
const metadata=spawnSync('cargo',['metadata','--manifest-path',resolve(root,'src-tauri/Cargo.toml'),'--locked','--filter-platform','x86_64-pc-windows-msvc','--format-version','1'],{encoding:'utf8',maxBuffer:32*1024*1024,cwd:root,windowsHide:true});
if(metadata.error||metadata.status!==0)throw new Error('Cargo metadata failed. Ensure Cargo is on PATH and run cargo fetch first.');
const graph=JSON.parse(metadata.stdout);const included=new Set(graph.resolve.nodes.map(n=>n.id));
for(const pkg of graph.packages){if(!included.has(pkg.id)||pkg.name==='audioharbor')continue;await collect(pkg.name,pkg.version,resolve(pkg.manifest_path,'..'),pkg.license,`https://crates.io/crates/${pkg.name}/${pkg.version}`);}
const output=resolve(root,'src-tauri/resources/engines');await mkdir(output,{recursive:true});
await writeFile(resolve(output,'APP_DEPENDENCY_LICENSES.txt'),notices.join('\n'));
await writeFile(resolve(output,'NOTICE_REVIEW.txt'),missing.length?`These dependencies do not ship a top-level license/notice file. Review the exact source and its embedded license before public distribution:\n${missing.join('\n')}\n`:'All included application dependencies have collected license/notice texts. Review engine sources separately.\n');
console.log(`Collected application dependency notices; ${missing.length} components need source-level notice review. See resources/engines/NOTICE_REVIEW.txt.`);
