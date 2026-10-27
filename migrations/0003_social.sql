-- Append-only social audit; existing financial records and legacy drafts are unchanged.
CREATE TABLE social_decisions (
    id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES agents(id),
    cycle_id uuid NOT NULL REFERENCES cycles(id), action_id uuid NOT NULL UNIQUE REFERENCES actions(id),
    data jsonb NOT NULL, disposition text NOT NULL CHECK(disposition IN ('draft','abstain','rejected')),
    review_status text NOT NULL DEFAULT 'requires_human_review' CHECK(review_status='requires_human_review'),
    publishing_enabled boolean NOT NULL DEFAULT false CHECK(NOT publishing_enabled),
    failure_code text, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX social_by_agent ON social_decisions(agent_id,created_at DESC,id);
