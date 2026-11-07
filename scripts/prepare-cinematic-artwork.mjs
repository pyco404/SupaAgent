/** Offline optimization only. Never calls a model, provider or remote asset host. */
import sharp from '../web/node_modules/sharp/dist/index.mjs';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';

const sections=['home','agents','intelligence','memory','economy','social','trading','factory','about'];
const source=new URL('../artifacts/cinematic-review/source/',import.meta.url);
const target=new URL('../reference-web/public/assets/',import.meta.url);
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const inventory=[];
for(const section of sections){
  const original=await readFile(new URL(section+'.png',source));
  const meta=await sharp(original).metadata();
  if(meta.format!=='png'||meta.width!==meta.height||meta.width<960||meta.pages>1)throw Error('square_original_required');
  const files=[];
  for(const [suffix,size,format,quality]of [['thumb',320,'webp',82],['480',480,'webp',82],['960',960,'webp',82],['960',960,'avif',48]]){
    const name=`supa-${section}-${suffix}.${format}`;
    const buffer=await sharp(original,{limitInputPixels:20_000_000}).resize(size,size,{fit:'contain'}).toColourspace('srgb')[format]({quality,effort:format==='avif'?5:4}).toBuffer();
    await writeFile(new URL(name,target),buffer);
    files.push({path:'reference-web/public/assets/'+name,width:size,height:size,bytes:buffer.length,sha256:digest(buffer)});
  }
  inventory.push({section,sourceSha256:digest(original),files});
}
await mkdir(new URL('../artifacts/cinematic-review/',import.meta.url),{recursive:true});
await writeFile(new URL('../artifacts/cinematic-review/asset-inventory.json',import.meta.url),JSON.stringify({generator:'built-in image_gen',transform:'Sharp 0.35.5 / offline resize and metadata removal',assets:inventory},null,2)+'\n');
const cells=await Promise.all(sections.map(async(section,index)=>({input:await sharp(new URL(section+'.png',source).pathname).resize(320,320).png().toBuffer(),left:(index%3)*320,top:Math.floor(index/3)*320})));
await sharp({create:{width:960,height:960,channels:3,background:'#080908'}}).composite(cells).png().toFile(new URL('../artifacts/cinematic-review/contact-sheet.png',import.meta.url).pathname);
console.log(JSON.stringify({sections:inventory.length,files:inventory.flatMap(x=>x.files).length,totalBytes:inventory.flatMap(x=>x.files).reduce((n,f)=>n+f.bytes,0)}));
