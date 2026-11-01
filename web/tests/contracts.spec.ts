import { expect, test } from "@playwright/test";
import { schemas, units, liveWorkConsentSchema } from "../src/lib/contracts";
import { agentId, agent, financials } from "./fixtures";

test("financial contracts reject unsafe integers and unlabelled or Mainnet values", () => {
  expect(schemas.financials.safeParse({ ...financials, simulated: false }).success).toBe(false);
  expect(schemas.financials.safeParse({ ...financials, balance_microusd: Number.MAX_SAFE_INTEGER + 1 }).success).toBe(false);
  expect(schemas.balances.safeParse({ network: "mainnet" }).success).toBe(false);
  expect(units(1234567890, 9)).toBe("1.23456789"); expect(units(-1000)).toBe("−0.001");
  expect(units(undefined)).toBe("Unavailable"); expect(units(Number.MAX_SAFE_INTEGER + 1)).toBe("Unavailable");
});
test("verified payment rows retain exact mint, network and evidence while stripping unknown data", () => {
  const proof = { network: "devnet", signature: "fixture-signature", mint: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU", amount_atoms: 1234567, slot: 42, evidence_source: "mock_rpc" };
  const row = { ...proof, id: agentId, data: proof };
  const [parsed] = schemas.payments.parse([row]);
  expect(parsed).toEqual(proof);
  expect(schemas.payments.safeParse([{ ...row, amount_atoms: 1234568 }]).success).toBe(false);
  expect(schemas.payments.safeParse([{ ...row, mint: "untrusted-mint" }]).success).toBe(false);
  expect(schemas.payments.safeParse([{ ...row, data: { ...proof, network: "mainnet" } }]).success).toBe(false);
});
test("API projections strip unrequested upstream fields", () => {
  const parsed = schemas.agent.parse({ agent: { ...agent, private_context: "not-public" }, financials, debug: "not-public" });
  expect(parsed).toEqual({ agent: { ...agent, personality: { traits: agent.personality.traits } }, financials });
  expect(parsed.agent.personality).not.toHaveProperty("voice");
});

test("work consent binds one unpaid job without frontend identity or authority fields", () => {
  const now = new Date();
  const input = {job_id:agentId,expected_revision:0,input_sha256:"a".repeat(64),approved_tools:["inspect_input","extract_evidence","check_requirements"],use_memory:false,consent:{request_id:agentId,goal:"Compare supplied documents",model:"gpt-6-luna",authorize_paid:true,consented_at:now.toISOString(),expires_at:new Date(now.getTime()+120000).toISOString(),max_requests:1,max_input_tokens:24192,max_output_tokens:1024,max_cost_microusd:4000}};
  expect(liveWorkConsentSchema.safeParse(input).success).toBe(true);
  expect(liveWorkConsentSchema.safeParse({...input,consent:{...input.consent,model:"claude-haiku-5-5"}}).success).toBe(true);
  for (const change of [{principal_id:agentId},{role:"owner"},{input_sha256:"bad"},{approved_tools:["transfer"]}]) {
    expect(liveWorkConsentSchema.safeParse({...input,...change}).success).toBe(false);
  }
  expect(liveWorkConsentSchema.safeParse({...input,consent:{...input.consent,max_requests:2}}).success).toBe(false);
  const disabled = {enabled:false,mode:"live_work_evaluation",project_id:null,continuous:false,commercial_delivery:false,policy:null,minimum_reservation_microusd:null,jobs:[],sessions:[]};
  expect(schemas["live-work"].safeParse(disabled).success).toBe(true);
  expect(schemas["live-work"].safeParse({...disabled,continuous:true}).success).toBe(false);
  expect(schemas["live-work"].safeParse({...disabled,commercial_delivery:true}).success).toBe(false);
});

test("Claude work records preserve provider origin and workspace without granting authority", () => {
  const stamp = new Date().toISOString();
  const consent = {request_id:agentId,goal:"Compare supplied documents",model:"claude-haiku-5-5",authorize_paid:true,consented_at:stamp,expires_at:stamp,max_requests:1,max_input_tokens:24192,max_output_tokens:1024,max_cost_microusd:4000};
  const authorization = {job_id:agentId,expected_revision:0,input_sha256:"a".repeat(64),approved_tools:["inspect_input","extract_evidence","check_requirements"],use_memory:false,consent};
  const policy = {model:"claude-haiku-5-5",live:true,budget_id:agentId,budget_limit_microusd:5000,input_rate:100000,cached_input_rate:10000,cache_write_rate:125000,output_rate:500000,input_token_bound:24192,max_output_tokens:1024};
  const session = {id:agentId,agent_id:agentId,approved_by:agentId,provider_scope:"wrkspc_synthetic_fixture",consent,policy,status:"failed",memory_ids:[],proposal:null,usage:null,reserved_microusd:3536,accounted_microusd:0,failure_code:"provider_http_401",metadata_requests:1,generation_requests:0,project_verified:false,tool_result:null,created_at:stamp,finished_at:stamp,work:{provider_evidence:"anthropic",authorization,request:{objective:"Compare supplied documents",material:"Supplied evidence",requirements:[],require_external_research:false},kind:"research_report",baseline:{},report:null,tool_results:null}};
  const envelope = {enabled:false,mode:"live_work_evaluation",project_id:"wrkspc_synthetic_fixture",continuous:false,commercial_delivery:false,policy:null,minimum_reservation_microusd:null,jobs:[],sessions:[session]};
  const parsed = schemas["live-work"].parse(envelope);
  expect(parsed.sessions[0].provider_scope).toBe("wrkspc_synthetic_fixture");
  expect(parsed.sessions[0].work?.provider_evidence).toBe("anthropic");
  expect(schemas["live-work"].safeParse({...envelope,sessions:[{...session,work:{...session.work,provider_evidence:"unverified"}}]}).success).toBe(false);
});
