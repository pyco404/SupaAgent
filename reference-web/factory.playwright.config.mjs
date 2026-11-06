import {defineConfig} from '../web/node_modules/@playwright/test/index.mjs';
import {fileURLToPath} from 'node:url';

const web=fileURLToPath(new URL('../web',import.meta.url));
const password='factory-fixture-only-'.repeat(3);
export default defineConfig({
 testDir:'./tests',testMatch:'factory-flow.browser.mjs',workers:1,retries:0,forbidOnly:true,
 timeout:90000,reporter:'list',outputDir:'../artifacts/factory-flow-20261106',
 use:{launchOptions:{executablePath:'/usr/bin/google-chrome'},reducedMotion:'reduce',serviceWorkers:'block'},
 projects:[{name:'desktop',use:{viewport:{width:1440,height:1000}}},
  {name:'mobile',use:{viewport:{width:390,height:844},isMobile:true,hasTouch:true}}],
 webServer:{
  command:`"${process.execPath}" node_modules/next/dist/bin/next dev --webpack --hostname 127.0.0.1 --port 3336`,
  cwd:web,url:'http://127.0.0.1:3336/healthz',timeout:120000,reuseExistingServer:false,
  env:{SUPA_SITE_MODE:'staging',SUPA_PUBLIC_LANDING_ENABLED:'true',
   SUPA_SITE_ORIGIN:'http://127.0.0.1:3336',SUPA_BACKEND_URL:'',SUPA_API_TOKEN:'',
   SUPA_STAGING_USERNAME:'visitor',SUPA_STAGING_PASSWORD:'factory-visitor-fixture-'.repeat(3),
   SUPA_WEB_OPERATORS:JSON.stringify(['viewer','operator'].map(name=>({
    username:name,password,api_token:('fixture-'+name+'-').repeat(4),
   }))),
  },
 },
});
