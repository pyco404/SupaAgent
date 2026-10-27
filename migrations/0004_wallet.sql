-- No signing material in PostgreSQL. These tables are separate from simulated and API spending ledgers.
CREATE TABLE wallet_identities (
 id uuid PRIMARY KEY, agent_id uuid NOT NULL UNIQUE REFERENCES agents(id),
 network text NOT NULL CHECK(network='devnet'), address text NOT NULL UNIQUE,
 owner_id uuid NOT NULL, origin jsonb NOT NULL CHECK(origin->>'kind'='human_operator'),
 created_at timestamptz NOT NULL DEFAULT now(),
 policy jsonb NOT NULL DEFAULT '{"locked":true,"reservations_enabled":false,"per_transaction_atoms":0,"daily_atoms":0,"destinations":[]}'
);
CREATE TABLE wallet_balances (
 wallet_id uuid PRIMARY KEY REFERENCES wallet_identities(id), data jsonb NOT NULL,
 slot bigint NOT NULL CHECK(slot>=0), observed_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE wallet_token_accounts (
 wallet_id uuid NOT NULL REFERENCES wallet_identities(id), address text NOT NULL, PRIMARY KEY(wallet_id,address)
);
CREATE TABLE verified_payments (
 id uuid PRIMARY KEY, wallet_id uuid NOT NULL REFERENCES wallet_identities(id),
 signature text NOT NULL, mint text NOT NULL, amount_atoms bigint NOT NULL CHECK(amount_atoms>0),
 slot bigint NOT NULL CHECK(slot>=0), evidence_source text NOT NULL CHECK(evidence_source IN ('mock_rpc','configured_rpc')),
 data jsonb NOT NULL, verified_at timestamptz NOT NULL DEFAULT now(), UNIQUE(wallet_id,signature,mint)
);
CREATE TABLE payment_checks (
 wallet_id uuid NOT NULL REFERENCES wallet_identities(id), signature text NOT NULL,
 status text NOT NULL CHECK(status IN ('verified','payment_unconfirmed','payment_invalid','rpc_failed','invalid_input')),
 checked_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(wallet_id,signature)
);
CREATE TABLE wallet_cursors (
 wallet_id uuid NOT NULL REFERENCES wallet_identities(id), address text NOT NULL,
 watermark text, page_before text, pending_head text, PRIMARY KEY(wallet_id,address)
);
CREATE TABLE operating_reservations (
 id uuid PRIMARY KEY, wallet_id uuid NOT NULL REFERENCES wallet_identities(id),
 destination text NOT NULL, amount_atoms bigint NOT NULL CHECK(amount_atoms>0),
 purpose text NOT NULL CHECK(purpose IN ('inference','infrastructure')),
 status text NOT NULL DEFAULT 'held' CHECK(status IN ('held','released')),
 created_at timestamptz NOT NULL DEFAULT now(), released_at timestamptz
);
CREATE TABLE financial_audit (
 id bigserial PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id),
 wallet_id uuid REFERENCES wallet_identities(id), operation text NOT NULL,
 outcome text NOT NULL, request_id uuid, actor_id uuid, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX financial_audit_agent ON financial_audit(agent_id,id DESC);
CREATE INDEX verified_payments_wallet ON verified_payments(wallet_id,slot);
CREATE INDEX operating_reservations_wallet ON operating_reservations(wallet_id,status);
