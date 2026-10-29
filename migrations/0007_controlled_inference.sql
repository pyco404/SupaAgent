-- Additive only: existing inference budgets and historical reservations are untouched.
CREATE TABLE live_sessions (
    id uuid PRIMARY KEY,
    agent_id uuid NOT NULL REFERENCES agents(id),
    approved_by uuid NOT NULL,
    budget_id uuid NOT NULL REFERENCES live_budgets(id),
    status text NOT NULL CHECK(status IN ('reserved','awaiting_approval','completed','failed','unknown')),
    lease_until timestamptz NOT NULL,
    data jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX live_sessions_agent ON live_sessions(agent_id,created_at DESC);
CREATE INDEX live_sessions_budget ON live_sessions(budget_id,status);
-- Session authorization and tool approval are separate, immutable audit entries.
CREATE TABLE live_session_audit (
    id bigserial PRIMARY KEY,
    session_id uuid NOT NULL REFERENCES live_sessions(id),
    actor uuid NOT NULL,
    operation text NOT NULL CHECK(operation IN ('authorize','settle','approve_tools','decline_tools')),
    created_at timestamptz NOT NULL DEFAULT now()
);
