CREATE TABLE agents (
    id uuid PRIMARY KEY,
    data jsonb NOT NULL,
    simulated_balance_microusd bigint NOT NULL DEFAULT 0 CHECK (simulated_balance_microusd >= 0),
    next_cycle_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE lifecycle_transitions (
    id bigserial PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id),
    from_state text, to_state text NOT NULL, reason text NOT NULL,
    actor text NOT NULL CHECK(actor IN ('operator','system')), created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE cycles (
    id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id),
    status text NOT NULL CHECK(status IN ('running','completed','failed','interrupted')),
    lease_until timestamptz NOT NULL, started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz, failure_code text
);
CREATE UNIQUE INDEX one_running_cycle ON cycles(agent_id) WHERE status='running';
CREATE TABLE actions (
    id uuid PRIMARY KEY, cycle_id uuid NOT NULL REFERENCES cycles(id),
    ordinal integer NOT NULL, proposal jsonb NOT NULL,
    authorization_outcome text NOT NULL CHECK(authorization_outcome IN ('pending','allowed','denied')),
    status text NOT NULL CHECK(status IN ('proposed','running','succeeded','denied','failed')),
    failure_code text, created_at timestamptz NOT NULL DEFAULT now(), finished_at timestamptz,
    UNIQUE(cycle_id,ordinal)
);
CREATE TABLE activity (
    id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id), cycle_id uuid REFERENCES cycles(id),
    kind text NOT NULL, code text NOT NULL, simulated boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX activity_by_agent ON activity(agent_id,created_at DESC,id);
CREATE TABLE simulated_ledger (
    id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id), cycle_id uuid REFERENCES cycles(id),
    kind text NOT NULL CHECK(kind IN ('seed','revenue','inference','operating')),
    amount_microusd bigint NOT NULL CHECK(amount_microusd <> 0),
    idempotency_key uuid NOT NULL, simulated boolean NOT NULL DEFAULT true CHECK(simulated),
    created_at timestamptz NOT NULL DEFAULT now(), UNIQUE(agent_id,idempotency_key),
    CHECK ((kind IN ('seed','revenue') AND amount_microusd>0) OR (kind IN ('inference','operating') AND amount_microusd<0))
);
CREATE TABLE tweet_drafts (
    id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id), cycle_id uuid NOT NULL REFERENCES cycles(id),
    action_id uuid NOT NULL UNIQUE REFERENCES actions(id), data jsonb NOT NULL,
    status text NOT NULL DEFAULT 'draft' CHECK(status='draft'), created_at timestamptz NOT NULL DEFAULT now()
);
