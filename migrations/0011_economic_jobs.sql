-- New isolated contractual accounting; never import treasury/budget/epoch history.
CREATE TABLE economic_services (
 id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id), spec jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE economic_jobs (
 id uuid PRIMARY KEY, service_id uuid NOT NULL REFERENCES economic_services(id),
 agent_id uuid NOT NULL REFERENCES agents(id), customer_id uuid NOT NULL,
 network text NOT NULL CHECK(network IN ('simulated','devnet')),
 spec jsonb NOT NULL, input text NOT NULL CHECK(octet_length(input) BETWEEN 1 AND 8000),
 state text NOT NULL CHECK(state IN ('created','awaiting_payment','funded','accepted','executing','completed','failed','cancelled','expired','disputed')),
 revision bigint NOT NULL DEFAULT 0 CHECK(revision>=0), attempts integer NOT NULL DEFAULT 0 CHECK(attempts BETWEEN 0 AND 2),
 received_atoms bigint NOT NULL DEFAULT 0 CHECK(received_atoms BETWEEN 0 AND 1000000000000),
 earned_atoms bigint NOT NULL DEFAULT 0 CHECK(earned_atoms>=0 AND earned_atoms<=received_atoms),
 expires_at timestamptz NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 lease_until timestamptz, execution_token uuid, output jsonb, output_sha256 text,
 failure_code text, assessment jsonb, request jsonb NOT NULL,
 UNIQUE(customer_id,id), CHECK((state='executing')=(execution_token IS NOT NULL AND lease_until IS NOT NULL)),
 CHECK(state<>'completed' OR (output IS NOT NULL AND output_sha256 ~ '^[0-9a-f]{64}$'))
);
CREATE INDEX economic_jobs_customer ON economic_jobs(customer_id,created_at DESC);
CREATE INDEX economic_jobs_agent ON economic_jobs(agent_id,created_at DESC);
CREATE TABLE economic_commands (
 actor uuid NOT NULL, request_id uuid NOT NULL, job_id uuid NOT NULL REFERENCES economic_jobs(id),
 request jsonb NOT NULL, result jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(actor,request_id)
);
CREATE TABLE economic_receipts (
 network text NOT NULL CHECK(network IN ('simulated','devnet')), signature text NOT NULL,
 job_id uuid NOT NULL REFERENCES economic_jobs(id), amount_atoms bigint NOT NULL CHECK(amount_atoms BETWEEN 1 AND 1000000000),
 evidence text NOT NULL CHECK(evidence IN ('simulated','mock_rpc','configured_rpc')),
 proof jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(network,signature),
 CHECK((network='simulated')=(evidence='simulated'))
);
CREATE TABLE economic_entries (
 id bigserial PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id), job_id uuid REFERENCES economic_jobs(id),
 network text NOT NULL CHECK(network IN ('simulated','devnet')), event_key text NOT NULL,
 debit text NOT NULL CHECK(debit IN ('cash','liability','inference_expense','infrastructure_expense','reserve')),
 credit text NOT NULL CHECK(credit IN ('cash','liability','revenue','reserve')),
 amount_atoms bigint NOT NULL CHECK(amount_atoms BETWEEN 1 AND 1000000000000),
 created_at timestamptz NOT NULL DEFAULT now(), UNIQUE(agent_id,network,event_key), CHECK(debit<>credit)
);
CREATE TABLE economic_spending_policies (
 agent_id uuid PRIMARY KEY REFERENCES agents(id), destinations text[] NOT NULL,
 ceiling_atoms bigint NOT NULL CHECK(ceiling_atoms BETWEEN 0 AND 1000000),
 reserve_atoms bigint NOT NULL CHECK(reserve_atoms BETWEEN 0 AND 1000000),
 approved_by uuid NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE economic_audit (
 id bigserial PRIMARY KEY, job_id uuid REFERENCES economic_jobs(id), agent_id uuid NOT NULL REFERENCES agents(id),
 actor uuid NOT NULL, operation text NOT NULL, data jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE FUNCTION economic_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'economic_history_is_immutable' USING ERRCODE='42501'; END;
$$;
CREATE TRIGGER economic_entries_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON economic_entries FOR EACH STATEMENT EXECUTE FUNCTION economic_immutable();
CREATE TRIGGER economic_receipts_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON economic_receipts FOR EACH STATEMENT EXECUTE FUNCTION economic_immutable();
CREATE TRIGGER economic_audit_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON economic_audit FOR EACH STATEMENT EXECUTE FUNCTION economic_immutable();
CREATE TRIGGER economic_commands_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON economic_commands FOR EACH STATEMENT EXECUTE FUNCTION economic_immutable();
CREATE TRIGGER economic_services_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON economic_services FOR EACH STATEMENT EXECUTE FUNCTION economic_immutable();
CREATE TABLE economic_holds (
 job_id uuid PRIMARY KEY REFERENCES economic_jobs(id), agent_id uuid NOT NULL REFERENCES agents(id),
 network text NOT NULL CHECK(network IN ('simulated','devnet')), amount_atoms bigint NOT NULL CHECK(amount_atoms BETWEEN 0 AND 1100),
 held boolean NOT NULL DEFAULT true
);
ALTER TABLE economic_entries DROP CONSTRAINT economic_entries_credit_check;
ALTER TABLE economic_entries ADD CHECK(credit IN ('cash','liability','revenue','reserve','capital'));
