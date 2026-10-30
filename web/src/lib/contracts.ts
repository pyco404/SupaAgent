import { z } from "zod";

const text = z.string().max(8000);
const id = z.uuid();
const integer = z.number().int().refine(Number.isSafeInteger);
const date = z.string().max(80);
const schedulePolicySchema=z.object({goal_ids:z.array(id).min(1).max(10).refine(ids=>new Set(ids).size===ids.length),interval_seconds:integer.min(5).max(86400),max_cycles:integer.min(1).max(100),simulated_budget_microusd:integer.min(1100).max(110000),valid_for_seconds:integer.min(60).max(86400)}).strict();
export const scheduleCommandSchema=z.object({request_id:id,expected_revision:integer.nonnegative(),operation:z.enum(["start","renew","pause","resume","stop"]),authorize_offline:z.literal(true),consented_at:z.iso.datetime().optional(),reason:z.string().trim().min(1).max(500),policy:schedulePolicySchema.nullable()}).strict().refine(value=>(["start","renew"].includes(value.operation))===(value.policy!==null)).refine(value=>value.operation!=="renew"||value.consented_at!==undefined);
const scheduleStatusSchema=z.object({agent_id:id,lifecycle:z.enum(["created","active","paused","suspended","terminated"]),revision:integer.nonnegative(),enabled:z.boolean(),policy:schedulePolicySchema.nullable(),cycles_started:integer.min(0).max(100),reserved_microusd:integer.min(0).max(110000),expires_at:date.nullable(),next_cycle_at:date,last_error:text.nullable(),mode:z.literal("offline"),simulated:z.literal(true),paid_requests_enabled:z.literal(false)});
export const autonomyRequestSchema = z.object({ request_id: id, goal: z.string().trim().min(3).max(500), authorize_offline: z.literal(true) }).strict();
const autonomyPolicySchema = z.object({ mode:z.literal("offline"),max_cycles:z.literal(3),simulated_seed_microusd:z.literal(10000),simulated_expense_limit_microusd:z.literal(3300),paid_requests:z.literal(0),tools:z.tuple([z.literal("read_memory"),z.literal("remember"),z.literal("evaluate_goal")]) });
const autonomyResultSchema = z.object({agent_id:id,mode:z.literal("offline"),simulated:z.literal(true),cycles:z.array(id).length(3).refine(ids=>new Set(ids).size===3),lifecycle:z.literal("paused"),paid_requests:z.literal(0),simulated_seed_microusd:z.literal(10000),simulated_expense_limit_microusd:z.literal(3300)});
export const goalSchema = z.object({ id, description: text, completed: z.boolean(), priority: integer, suggested_progress_bps: integer });
export const memorySchema = z.object({ id, lesson: text, kind: z.enum(["hypothesis", "verified_observation"]), source_event_ids: z.array(id), cycle_id: id.nullable(), created_at: date });
export const agentSchema = z.object({
  id, name: text, lifecycle: z.enum(["created", "active", "paused", "suspended", "terminated"]),
  personality: z.object({ traits: z.array(text) }), capabilities: z.array(text),
  goals: z.array(goalSchema), memory: z.array(memorySchema), created_at: date,
  config: z.object({ interval_seconds: integer, max_steps: integer }),
});
export const financialSchema = z.object({
  simulated: z.literal(true), unit: z.literal("simulated_micro_usd"), balance_microusd: integer,
  revenue_microusd: integer, inference_microusd: integer, operating_microusd: integer,
  runway_days: z.number().finite().nullable(), runway_assumption: text,
});
export const eventSchema = z.object({ id, cycle_id: id.nullable(), kind: text, code: text, simulated: z.boolean(), created_at: date });
const cycleSchema = z.object({ id, status: text, started_at: date, finished_at: date.nullable(), failure_code: text.nullable() });
const actionSchema = z.object({ id, cycle_id: id, ordinal: integer, proposal: z.object({ tool: text }), authorization_outcome: text, status: text, failure_code: text.nullable() });
const ledgerSchema = z.object({ id, kind: text, amount_microusd: integer, simulated: z.literal(true), created_at: date });
const socialData = z.object({ text: text.nullable().optional(), choice: text.optional(), style: text.nullable().optional(), reason: text.optional(), event_ids: z.array(id), memory_ids: z.array(id).optional(), claims: z.array(z.object({ event_id: id, fact: text, wording: text.optional() })).optional() });
const socialSchema = z.object({ id, cycle_id: id.nullable().optional(), data: socialData, disposition: text, failure_code: text.nullable(), created_at: date });
const draftSchema = z.object({ id, data: socialData, created_at: date });
const balanceSchema = z.object({ evidence_source: z.enum(["configured_rpc", "mock_rpc"]), network: z.literal("devnet"), address: text, slot: integer, sol_lamports: integer, usdc_atoms: integer });
const treasurySchema = z.object({ network: z.literal("devnet"), environment: text, observed_sol_lamports: integer, observed_usdc_atoms: integer, verified_incoming_usdc_atoms: integer, reserved_usdc_atoms: integer, available_operating_usdc_atoms: integer, api_inference_estimate_microusd: integer, fresh: z.boolean(), reconciled: z.boolean(), observed_at: date.nullable() });
const paymentSchema = z.object({
  signature: text, mint: z.literal("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"), amount_atoms: integer.positive(), slot: integer.nonnegative(),
  evidence_source: z.enum(["configured_rpc", "mock_rpc"]),
  data: z.object({ network: z.literal("devnet"), signature: text, mint: text, amount_atoms: integer, slot: integer, evidence_source: z.enum(["configured_rpc", "mock_rpc"]) }),
}).refine(row => row.signature === row.data.signature && row.mint === row.data.mint && row.amount_atoms === row.data.amount_atoms && row.slot === row.data.slot && row.evidence_source === row.data.evidence_source)
  .transform(({ data, ...row }) => ({ ...row, network: data.network }));
export const liveConsentSchema=z.object({request_id:id,goal:z.string().trim().min(1).max(500),model:z.string().min(1).max(100),authorize_paid:z.literal(true),consented_at:z.iso.datetime(),expires_at:z.iso.datetime(),max_requests:z.literal(1),max_input_tokens:integer.min(9000).max(24192),max_output_tokens:integer.min(256).max(4096),max_cost_microusd:integer.min(1).max(100000)}).strict();
export const liveApprovalSchema=z.object({approve:z.boolean(),confirm_review:z.literal(true)}).strict();
const livePolicySchema=z.object({model:text,live:z.literal(true),budget_id:id,budget_limit_microusd:integer,input_token_bound:integer,max_output_tokens:integer});
export const liveSessionSchema=z.object({id,agent_id:id,approved_by:id,consent:liveConsentSchema,policy:livePolicySchema,status:z.enum(['reserved','awaiting_approval','completed','failed','unknown']),memory_ids:z.array(id).max(12),proposal:z.object({summary:text,memory_ids:z.array(id).max(12),actions:z.array(z.object({tool:z.literal('read_memory'),event_ids:z.array(id).length(0),lesson:z.null(),evaluation:z.null()})).max(1)}).nullable(),usage:z.object({input_tokens:integer,output_tokens:integer,cached_input_tokens:integer,cache_write_tokens:integer}).nullable(),reserved_microusd:integer,accounted_microusd:integer,failure_code:text.nullable(),metadata_requests:integer,generation_requests:integer,project_verified:z.boolean(),tool_result:z.object({result:text,executed_actions:integer}).passthrough().nullable(),created_at:date,finished_at:date.nullable()});
export const economicSpecSchema=z.object({name:z.string().min(1).max(80),kind:z.enum(['summary','research_report','code_analysis','api_report','delegation_simulation']),price_atoms:integer.min(2000).max(1000000000),estimated_inference_atoms:z.literal(1000),estimated_infrastructure_atoms:z.literal(100),max_input_bytes:integer.min(1).max(8000),max_attempts:integer.min(1).max(2)}).strict();
const economicServiceSchema=z.object({id,agent_id:id,spec:economicSpecSchema,created_at:date});
export const economicJobSchema=z.object({id,service_id:id,agent_id:id,customer_id:id,network:z.enum(['simulated','devnet']),spec:economicSpecSchema,input:z.string().max(8000),state:z.enum(['created','awaiting_payment','funded','accepted','executing','completed','failed','cancelled','expired','disputed']),revision:integer.nonnegative(),attempts:integer.min(0).max(2),received_atoms:integer.nonnegative(),earned_atoms:integer.nonnegative(),expires_at:date,created_at:date,lease_until:date.nullable(),output:z.record(z.string(),z.unknown()).nullable(),output_sha256:z.string().regex(/^[a-f0-9]{64}$/).nullable(),failure_code:text.nullable(),assessment:z.record(z.string(),z.unknown()).nullable()});
export const economicCreateSchema=z.object({request_id:id,service_id:id,network:z.enum(['simulated','devnet']),input:z.string().min(1).max(8000),valid_for_seconds:integer.min(60).max(86400)}).strict();
export const economicCommandSchema=z.object({request_id:id,expected_revision:integer.nonnegative(),operation:z.enum(['request_payment','cancel','dispute','expire','retry','recover','accept','execute']),authorize:z.literal(true)}).strict();
export const economicVerifySchema=z.object({signature:z.string().regex(/^[1-9A-HJ-NP-Za-km-z]{64,90}$/),authorize_read_only_verification:z.literal(true)}).strict();
export const economicPaymentSchema=z.object({request_id:id,amount_atoms:integer.min(1).max(1000000000),authorize_simulated:z.literal(true)}).strict();
const economicAccountsSchema=z.object({agent_id:id,mainnet_enabled:z.literal(false),signing_enabled:z.literal(false),networks:z.array(z.object({network:z.enum(['simulated','devnet']),cash_atoms:integer,liability_atoms:integer,earned_atoms:integer,inference_atoms:integer,infrastructure_atoms:integer,available_atoms:integer,net_result_atoms:integer,runway_days:z.null(),runway_reason:text})).length(2),entries:z.array(z.record(z.string(),z.unknown())).max(200),audit:z.array(z.record(z.string(),z.unknown())).max(200),policy:z.record(z.string(),z.unknown()).nullable()});
export const schemas = {
  jobs:z.object({jobs:z.array(economicJobSchema).max(100)}),
  "job-payment":z.object({job:economicJobSchema,refund_eligible_atoms:integer.nonnegative(),payment_request:z.object({network:z.enum(['simulated','devnet']),amount_atoms:integer,decimals:z.literal(6),mint:z.string().nullable(),recipient:z.string().nullable(),memo:id,expires_at:date,signing_enabled:z.literal(false)})}),
  services:z.object({services:z.array(economicServiceSchema).max(100),mainnet_enabled:z.literal(false)}),
  economy:z.object({accounts:economicAccountsSchema,services:z.array(economicServiceSchema).max(100),jobs:z.array(economicJobSchema).max(100)}),
  live:z.object({enabled:z.boolean(),mode:z.literal("controlled_live"),continuous:z.literal(false),policy:livePolicySchema.nullable(),minimum_reservation_microusd:integer.nullable(),sessions:z.array(liveSessionSchema).max(20)}),
  access:z.object({role:z.enum(["owner","operator","viewer"])}),
  "execution-health":z.object({agent_id:id,status:z.enum(["healthy","delayed","unavailable","recovery_required"]),execution_running:z.boolean(),worker_enabled:z.boolean().optional(),lifecycle:text,checked_at:date,heartbeat_at:date.nullable(),tick_finished_at:date.nullable(),lease_until:date.nullable(),cycle_id:id.nullable()}),
  authorizations:z.array(z.object({id,approved_by:id.nullable(),approved_at:date.nullable(),policy:schedulePolicySchema,expires_at:date,closed_at:date.nullable(),cycles_started:integer.nonnegative(),reserved_microusd:integer.nonnegative(),legacy:z.boolean()})).max(100),
  directory:z.array(agentSchema.pick({id:true,name:true,lifecycle:true,created_at:true})).max(100),
  schedule:scheduleStatusSchema,
  autonomy:autonomyPolicySchema, "autonomy-result":autonomyResultSchema,
  health: z.object({ status: text, economic_mode: z.literal("simulation"), publishing_enabled: z.literal(false).optional(),
    epoch:z.object({id,accounting_namespace:id,label:z.string().min(1).max(80),kind:z.literal("development"),state:z.enum(["prepared","active_read_only"]),historical_database:z.literal("unrecovered"),revision:integer.nonnegative().optional(),approval_id:id.nullable().optional()}).nullable().optional(),
    historical_database:z.literal("unrecovered").optional(),
  }),
  agents: z.array(agentSchema).max(100),
  agent: z.object({ agent: agentSchema, financials: financialSchema }),
  financials: financialSchema, memory: z.array(memorySchema), goals: z.array(goalSchema),
  activity: z.array(eventSchema), cycles: z.array(cycleSchema), actions: z.array(actionSchema),
  "simulated-ledger": z.array(ledgerSchema), social: z.array(socialSchema), drafts: z.array(draftSchema),
  wallet: z.object({ address: text, network: z.literal("devnet") }),
  balances: balanceSchema.nullable(), treasury: treasurySchema, payments: z.array(paymentSchema),
};
export type Agent = z.infer<typeof agentSchema>;
export type Financials = z.infer<typeof financialSchema>;
export type Memory = z.infer<typeof memorySchema>;
export type Event = z.infer<typeof eventSchema>;
export type Cycle = z.infer<typeof cycleSchema>;
export type Action = z.infer<typeof actionSchema>;
export type Ledger = z.infer<typeof ledgerSchema>;
export type Social = z.infer<typeof socialSchema>;
export type Draft = z.infer<typeof draftSchema>;
export type Balance = z.infer<typeof balanceSchema>;
export type Treasury = z.infer<typeof treasurySchema>;
export type Payment = z.infer<typeof paymentSchema>;

export function units(value: number | undefined, decimals = 6): string {
  if (value === undefined || !Number.isSafeInteger(value)) return "Unavailable";
  const amount = BigInt(value), scale = 10n ** BigInt(decimals), abs = amount < 0n ? -amount : amount;
  const fraction = (abs % scale).toString().padStart(decimals, "0").replace(/0+$/, "");
  return `${amount < 0n ? "−" : ""}${(abs / scale).toLocaleString("en-US")}${fraction ? `.${fraction}` : ""}`;
}
