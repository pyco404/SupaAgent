import { z } from "zod";

const text = z.string().max(8000);
const id = z.uuid();
const integer = z.number().int().refine(Number.isSafeInteger);
const date = z.string().max(80);
export const goalSchema = z.object({ id, description: text, completed: z.boolean(), priority: integer, suggested_progress_bps: integer });
export const memorySchema = z.object({ id, lesson: text, kind: z.enum(["hypothesis", "verified_observation"]), source_event_ids: z.array(id), cycle_id: id.nullable(), created_at: date });
export const agentSchema = z.object({
  id, name: text, lifecycle: z.enum(["created", "active", "paused", "suspended", "terminated"]),
  personality: z.object({ traits: z.array(text), voice: text }), capabilities: z.array(text),
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
export const schemas = {
  health: z.object({ status: text, economic_mode: z.literal("simulation"), publishing_enabled: z.literal(false).optional() }),
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
