CREATE TABLE live_budgets (
    id uuid PRIMARY KEY,
    limit_microusd bigint NOT NULL CHECK(limit_microusd BETWEEN 1 AND 1000000),
    committed_microusd bigint NOT NULL DEFAULT 0 CHECK(committed_microusd>=0),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE inference_calls (
    id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id), cycle_id uuid NOT NULL REFERENCES cycles(id),
    purpose text NOT NULL CHECK(purpose IN ('plan','draft')), attempt integer NOT NULL,
    policy jsonb NOT NULL, budget_id uuid REFERENCES live_budgets(id),
    reserved_microusd bigint NOT NULL CHECK(reserved_microusd>=0),
    accounted_microusd bigint NOT NULL CHECK(accounted_microusd>=0),
    usage jsonb, usage_source text NOT NULL DEFAULT 'unavailable',
    status text NOT NULL CHECK(status IN ('reserved','succeeded','failed','unknown','rejected')),
    failure_code text, created_at timestamptz NOT NULL DEFAULT now(), finished_at timestamptz
);
CREATE INDEX inference_by_cycle ON inference_calls(cycle_id,created_at);
CREATE TABLE goal_evaluations (
    id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id), cycle_id uuid NOT NULL REFERENCES cycles(id),
    action_id uuid UNIQUE NOT NULL REFERENCES actions(id), data jsonb NOT NULL,
    evidence_event_ids uuid[] NOT NULL,
    verification text NOT NULL DEFAULT 'model_assessment' CHECK(verification='model_assessment'),
    created_at timestamptz NOT NULL DEFAULT now()
);
-- Upgrade legacy memories explicitly as hypotheses through serde defaults when loaded.
-- Financial simulation tables remain unchanged; real provider budget records are separate.
