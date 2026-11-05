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
-- Administrator-provisioned policy only. No HTTP or runtime INSERT grant.
-- A newer disabled policy version revokes earlier reviewer authority.
CREATE TABLE ecosystem_social.qualification_policies (
 id uuid NOT NULL, version integer NOT NULL CHECK(version>0),
 enabled boolean NOT NULL DEFAULT false,
 reviewers uuid[] NOT NULL CHECK(cardinality(reviewers)>0 AND array_position(reviewers,NULL) IS NULL
 AND NOT ('00000000-0000-0000-0000-000000000000'::uuid=ANY(reviewers))),
 max_validity_seconds integer NOT NULL CHECK(max_validity_seconds BETWEEN 1 AND 604800),
 PRIMARY KEY(id,version)
);
CREATE TABLE ecosystem_social.owner_reviews (
 id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES public.agents(id),
 creator_id uuid NOT NULL, kind text NOT NULL CHECK(kind IN ('qualification','achievement')),
 target_id uuid NOT NULL, body jsonb NOT NULL CHECK(jsonb_typeof(body)='object'),
 recorded_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE ecosystem_social.sources (
 source_id uuid PRIMARY KEY, agent_id uuid NOT NULL REFERENCES public.agents(id),
 kind text NOT NULL CHECK(kind IN ('agent_created','task_completed','task_failed','agent_qualified','achievement')), snapshot jsonb NOT NULL,
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
CREATE TRIGGER qualification_policies_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON ecosystem_social.qualification_policies
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
CREATE TRIGGER owner_reviews_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON ecosystem_social.owner_reviews
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
-- Locks serialize existing ACL mutations without granting UPDATE on agents/ACL.
-- No mutation, financial access or dynamic SQL. Search path cannot be hijacked.
CREATE FUNCTION ecosystem_social.lock_agent(target uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM public.application_epoch) THEN
  RAISE EXCEPTION 'ecosystem_epoch_not_authorized' USING ERRCODE='42501';
 END IF;
 PERFORM 1 FROM public.agents WHERE id=target FOR UPDATE;
END;
$$;
REVOKE ALL ON FUNCTION ecosystem_social.lock_agent(uuid) FROM PUBLIC;
COMMIT;
