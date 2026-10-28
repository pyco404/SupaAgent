-- No existing identity is authorized or changed by this migration.
CREATE TABLE agent_schedules (
    agent_id uuid PRIMARY KEY REFERENCES agents(id),
    revision bigint NOT NULL DEFAULT 1 CHECK(revision > 0),
    enabled boolean NOT NULL DEFAULT false,
    policy jsonb NOT NULL,
    cycles_started bigint NOT NULL DEFAULT 0 CHECK(cycles_started BETWEEN 0 AND 100),
    reserved_microusd bigint NOT NULL DEFAULT 0 CHECK(reserved_microusd BETWEEN 0 AND 110000),
    expires_at timestamptz NOT NULL,
    last_error text,
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE schedule_commands (
    request_id uuid PRIMARY KEY,
    agent_id uuid NOT NULL REFERENCES agents(id),
    command jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX authorized_schedule_lookup ON agent_schedules(agent_id) WHERE enabled;
