-- Additive only. Existing identities remain unowned until explicitly assigned offline.
CREATE TABLE agent_access (
 agent_id uuid NOT NULL REFERENCES agents(id), principal_id uuid NOT NULL,
 role text NOT NULL CHECK(role IN ('owner','operator','viewer')),
 granted_by uuid NOT NULL, granted_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(agent_id,principal_id)
);
CREATE UNIQUE INDEX one_agent_owner ON agent_access(agent_id) WHERE role='owner';
CREATE TABLE access_audit (
 id bigserial PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id),
 principal_id uuid NOT NULL, role text NOT NULL, approved_by uuid NOT NULL,
 approved_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE schedule_commands ADD COLUMN approved_by uuid;
CREATE TABLE schedule_authorizations (
 id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id),
 approved_by uuid, approved_at timestamptz DEFAULT now(),
 policy jsonb NOT NULL, expires_at timestamptz NOT NULL,
 closed_at timestamptz, cycles_started bigint NOT NULL DEFAULT 0,
 reserved_microusd bigint NOT NULL DEFAULT 0,
 legacy boolean NOT NULL DEFAULT false
);
CREATE UNIQUE INDEX one_open_authorization ON schedule_authorizations(agent_id) WHERE closed_at IS NULL;
-- Preserve previous authorization and allowance evidence without inventing an approver.
INSERT INTO schedule_authorizations(id,agent_id,approved_at,policy,expires_at,cycles_started,reserved_microusd,legacy)
 SELECT s.agent_id,s.agent_id,(SELECT min(c.created_at) FROM schedule_commands c WHERE c.agent_id=s.agent_id AND c.command->>'operation'='start'),s.policy,s.expires_at,s.cycles_started,s.reserved_microusd,true FROM agent_schedules s;
ALTER TABLE agent_schedules ADD COLUMN authorization_id uuid REFERENCES schedule_authorizations(id);
UPDATE agent_schedules SET authorization_id=agent_id;
CREATE TABLE worker_health (
 id uuid PRIMARY KEY, kind text NOT NULL CHECK(kind IN ('api','worker')),
 enabled boolean NOT NULL, poll_seconds bigint NOT NULL CHECK(poll_seconds BETWEEN 1 AND 3600),
 started_at timestamptz NOT NULL DEFAULT now(), heartbeat_at timestamptz NOT NULL DEFAULT now(),
 tick_started_at timestamptz, tick_finished_at timestamptz, last_error text,
 stopped_at timestamptz
);
ALTER TABLE cycles ADD COLUMN authorization_id uuid REFERENCES schedule_authorizations(id);
ALTER TABLE cycles ADD COLUMN worker_id uuid REFERENCES worker_health(id);
CREATE INDEX worker_lease_lookup ON cycles(worker_id) WHERE status='running';
