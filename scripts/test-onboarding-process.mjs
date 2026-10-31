#!/usr/bin/env node
/** Real child-process regressions; no databases, credentials or agent execution. */
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import test from 'node:test';
import {stopFixture} from './onboarding-smoke.mjs';
async function child(code){const p=spawn(process.execPath,['-e',code],{env:{PATH:process.env.PATH},stdio:['ignore','pipe','ignore']});await once(p.stdout,'data');return p;}

test('a signal-terminated child can be stopped repeatedly without waiting again',async()=>{
 const p=await child("console.log('ready');setInterval(()=>{},1000)");
 await stopFixture(p);assert.equal(p.signalCode,'SIGTERM');assert.equal(p.exitCode,null);
 await stopFixture(p);
});
test('graceful exit and already completed children are accepted',async()=>{
 const p=await child("process.on('SIGTERM',()=>process.exit(0));console.log('ready');setInterval(()=>{},1000)");
 await stopFixture(p);assert.equal(p.exitCode,0);await stopFixture(p);
});
test('an unresponsive owned fixture is killed and reported as unsuccessful cleanup',async()=>{
 const p=await child("process.on('SIGTERM',()=>{});console.log('ready');setInterval(()=>{},1000)");
 await assert.rejects(()=>stopFixture(p,50),/fixture_forced_shutdown/);
 assert.equal(p.signalCode,'SIGKILL');await stopFixture(p);
});
