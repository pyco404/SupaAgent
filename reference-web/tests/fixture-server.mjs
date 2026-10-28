/** Synthetic read-only HTTP fixtures. Never uses a real database or provider. */
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { records, agent, agentId, eventId, timestamp, financials } from '../../web/tests/fixtures.ts';
const token='fixture-token-'.repeat(4);
const fixtures={...records,
  directory:[{id:agentId,name:agent.name,lifecycle:agent.lifecycle,created_at:agent.created_at}],
  [`agents/${agentId}/access`]:{role:'owner'},
  [`agents/${agentId}/authorizations`]:[],
  [`agents/${agentId}/execution-health`]:{agent_id:agentId,status:'unavailable',execution_running:false,lifecycle:agent.lifecycle,checked_at:timestamp,heartbeat_at:null,tick_finished_at:null,lease_until:null,cycle_id:null},
  [`agents/${agentId}/goals`]:agent.goals,
  [`agents/${agentId}/financials`]:financials,
  [`agents/${agentId}/drafts`]:[{id:eventId,created_at:timestamp,data:{text:'TEST FIXTURE: My simulated budget has boundaries. So does my enthusiasm for invoices.',style:'humor',event_ids:[eventId],memory_ids:[]}}],
};
const fixture=createServer((request,response)=>{
  response.setHeader('Content-Type','application/json');
  if(request.method!=='GET'||request.headers.authorization!==`Bearer ${token}`){response.writeHead(403).end('{}');return;}
  const value=fixtures[request.url.slice(1)];
  if(value===undefined){response.writeHead(404).end('{}');return;}
  response.end(JSON.stringify(value));
});
await new Promise(resolve=>fixture.listen(18082,'127.0.0.1',resolve));
const child=spawn(process.execPath,['server.mjs'],{cwd:new URL('..',import.meta.url),env:{...process.env,PORT:'3202',SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:'http://127.0.0.1:18082',SUPA_API_TOKEN:token,SUPA_WEB_USERNAME:'fixture',SUPA_WEB_PASSWORD:'fixture-password-'.repeat(3)},stdio:'ignore'});
function stop(){child.kill();fixture.close(()=>process.exit(0));}
process.on('SIGTERM',stop);process.on('SIGINT',stop);child.on('error',()=>{fixture.close();process.exitCode=1;});
