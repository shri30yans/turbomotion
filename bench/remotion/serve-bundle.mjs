// Keep static assets available throughout multi-scene browser QA and rendering.
import {createServer} from 'node:http';
import {createReadStream,statSync} from 'node:fs';
import {resolve,extname,sep} from 'node:path';
export async function serveBundle(directory){
 const root=resolve(directory);
 const mime={'.html':'text/html','.js':'text/javascript','.css':'text/css','.json':'application/json','.jpg':'image/jpeg','.png':'image/png','.svg':'image/svg+xml','.ttf':'font/ttf','.wav':'audio/wav','.mp4':'video/mp4','.vtt':'text/vtt'};
 const server=createServer((req,res)=>{
  try{
   const path=resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname));
   if(path!==root&&!path.startsWith(root+sep)){res.writeHead(403);res.end();return}
   const file=path===root?resolve(root,'index.html'):path;
   const stat=statSync(file);if(!stat.isFile())throw new Error('Not a file');
   const range=req.headers.range?.match(/^bytes=(\d+)-(\d*)$/);
   const start=range?Number(range[1]):0,end=range&&range[2]?Math.min(Number(range[2]),stat.size-1):stat.size-1;
   if(start>end||start>=stat.size){res.writeHead(416);res.end();return}
   res.writeHead(range?206:200,{'Content-Type':mime[extname(file)]||'application/octet-stream','Content-Length':end-start+1,'Accept-Ranges':'bytes',...(range?{'Content-Range':`bytes ${start}-${end}/${stat.size}`}:{})});
   if(req.method==='HEAD'){res.end();return}
   createReadStream(file,{start,end}).pipe(res);
  }catch{res.writeHead(404);res.end()}
 });
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 return {url:`http://127.0.0.1:${server.address().port}`,close:()=>new Promise(r=>server.close(r))};
}
