-- Explicit, separately installed feature extension. NOT an automatic migration.
-- Install only in a reviewed disposable database for now. No runtime grants.
BEGIN;
CREATE SCHEMA ecosystem_social;
REVOKE ALL ON SCHEMA ecosystem_social FROM PUBLIC;
CREATE TABLE ecosystem_social.purposes (
 id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES public.agents(id),
 creator_id uuid NOT NULL, revision bigint NOT NULL CHECK(revision>0), body jsonb NOT NULL,
 UNIQUE(agent_id,revision)
);
CREATE TABLE ecosystem_social.consents (
 request_id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES public.agents(id),
 creator_id uuid NOT NULL, revision bigint NOT NULL CHECK(revision>0), body jsonb NOT NULL,
 recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(), UNIQUE(agent_id,revision)
);
CREATE TABLE ecosystem_social.sources (
 source_id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES public.agents(id),
 kind text NOT NULL CHECK(kind='agent_created'), snapshot jsonb NOT NULL,
 recorded_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE ecosystem_social.preparations (
 request_id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES public.agents(id), actor uuid NOT NULL,
 consent_revision bigint NOT NULL CHECK(consent_revision>0), payload jsonb NOT NULL,
 payload_sha256 text NOT NULL CHECK(payload_sha256 ~ '^[0-9a-f]{64}$'),
 status text NOT NULL DEFAULT 'prepared_requires_review' CHECK(status='prepared_requires_review'),
 publishing_enabled boolean NOT NULL DEFAULT false CHECK(NOT publishing_enabled),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(), expires_at timestamptz NOT NULL,
 CHECK(expires_at>created_at),
 CHECK((jsonb_typeof(payload)='object' AND jsonb_typeof(payload->'events')='array'
   AND payload->>'schema'='public_ecosystem_events_v1'
   AND payload->'publishing_enabled'='false'::jsonb AND payload->'requires_semantic_review'='true'::jsonb) IS TRUE),
 CHECK(payload_sha256=encode(sha256(convert_to(public.work_review_canonical(payload),'UTF8')),'hex'))
);
CREATE TABLE ecosystem_social.prepared_events (
 source_id uuid PRIMARY KEY REFERENCES ecosystem_social.sources(source_id),
 request_id uuid NOT NULL REFERENCES ecosystem_social.preparations(request_id)
);
REVOKE ALL ON ALL TABLES IN SCHEMA ecosystem_social FROM PUBLIC;
CREATE FUNCTION ecosystem_social.immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'ecosystem_social_history_immutable' USING ERRCODE='42501'; END;
$$;
REVOKE ALL ON FUNCTION ecosystem_social.immutable() FROM PUBLIC;
CREATE TRIGGER purposes_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON ecosystem_social.purposes
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
CREATE TRIGGER consents_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON ecosystem_social.consents
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
CREATE TRIGGER sources_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON ecosystem_social.sources
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
CREATE TRIGGER preparations_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON ecosystem_social.preparations
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
CREATE TRIGGER prepared_events_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON ecosystem_social.prepared_events
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
COMMIT;
