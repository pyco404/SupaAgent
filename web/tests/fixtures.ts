// Synthetic browser-test data only. Never loaded by the application or a live database.
export const agentId = "11111111-1111-4111-8111-111111111111";
export const eventId = "22222222-2222-4222-8222-222222222222";
export const cycleId = "33333333-3333-4333-8333-333333333333";
export const memoryId = "44444444-4444-4444-8444-444444444444";
export const timestamp = "2026-10-28T08:00:00Z";
export const financials = { simulated: true, unit: "simulated_micro_usd", balance_microusd: 2500000, revenue_microusd: 0, inference_microusd: 1000, operating_microusd: 500, runway_days: null, runway_assumption: "Insufficient recorded simulated expense history." };
export const memory = { id: memoryId, lesson: "Browser-test hypothesis: conserve simulated resources.", kind: "hypothesis", source_event_ids: [eventId], cycle_id: cycleId, created_at: timestamp };
export const agent = { id: agentId, name: "Fixture Supa", lifecycle: "paused", personality: { traits: ["curious", "witty"], voice: "A synthetic identity used only by browser tests." }, capabilities: ["remember", "draft_tweet"], config: { interval_seconds: 60, max_steps: 4 }, created_at: timestamp, memory: [memory], goals: [{ id: eventId, description: "Test a bounded reasoning loop", completed: false, priority: 8, suggested_progress_bps: 2500 }] };
export const records: Record<string, unknown> = {
  health: { status: "ok", economic_mode: "simulation", publishing_enabled: false }, agents: [agent],
  [`agents/${agentId}`]: { agent, financials },
  [`agents/${agentId}/memory`]: [memory, { ...memory, id: eventId, lesson: "Browser-test verified observation: a simulated expense was recorded.", kind: "verified_observation" }],
  [`agents/${agentId}/activity`]: [{ id: eventId, cycle_id: cycleId, kind: "cycle_completed", code: "completed", simulated: true, created_at: timestamp }],
  [`agents/${agentId}/cycles`]: [{ id: cycleId, status: "completed", started_at: timestamp, finished_at: timestamp, failure_code: null }],
  [`agents/${agentId}/actions`]: [{ id: eventId, cycle_id: cycleId, ordinal: 0, proposal: { tool: "remember" }, authorization_outcome: "allowed", status: "completed", failure_code: null }],
  [`agents/${agentId}/simulated-ledger`]: [{ id: eventId, kind: "inference", amount_microusd: -1000, simulated: true, created_at: timestamp }],
  [`agents/${agentId}/social`]: [
    { id: eventId, cycle_id: cycleId, disposition: "draft", failure_code: null, created_at: timestamp, data: { choice: "draft", text: "TEST FIXTURE: My simulated budget has boundaries. So does my enthusiasm for invoices.", style: "humor", reason: "worth_sharing", event_ids: [eventId], memory_ids: [memoryId], claims: [] } },
    { id: memoryId, cycle_id: cycleId, disposition: "abstain", failure_code: null, created_at: timestamp, data: { choice: "abstain", text: null, style: null, reason: "nothing_new", event_ids: [], memory_ids: [] } },
    { id: cycleId, cycle_id: cycleId, disposition: "rejected", failure_code: "social_repetition", created_at: timestamp, data: { choice: "draft", text: "Rejected fixture text must not be shown as an approved draft", style: "humor", reason: "too_repetitive", event_ids: [eventId], memory_ids: [] } },
  ],
  [`agents/${agentId}/drafts`]: [],
  [`agents/${agentId}/wallet/balances`]: null,
  [`agents/${agentId}/wallet/treasury`]: { network: "devnet", environment: "mocked_devnet", observed_sol_lamports: 0, observed_usdc_atoms: 0, verified_incoming_usdc_atoms: 0, reserved_usdc_atoms: 0, available_operating_usdc_atoms: 0, api_inference_estimate_microusd: 0, fresh: false, reconciled: false, observed_at: null },
  [`agents/${agentId}/wallet/payments`]: [],
};
