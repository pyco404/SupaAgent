-- Explicit synthetic-demo extension. Never applied by automatic migrations.
-- Requires the separately reviewed ecosystem-social extension. No live grants.
BEGIN;
CREATE SCHEMA agent_factory;
REVOKE ALL ON SCHEMA agent_factory FROM PUBLIC;
CREATE TABLE agent_factory.submissions (
 creator_id uuid NOT NULL, request_id uuid NOT NULL,
 agent_id uuid NOT NULL UNIQUE REFERENCES public.agents(id),
 configuration jsonb NOT NULL CHECK(jsonb_typeof(configuration)='object'),
 recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(creator_id,request_id)
);
REVOKE ALL ON agent_factory.submissions FROM PUBLIC;
CREATE TRIGGER submissions_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
 ON agent_factory.submissions FOR EACH STATEMENT
 EXECUTE FUNCTION ecosystem_social.immutable();
-- Trusted application supplies the authenticated actor. The login receives only
-- these EXECUTE rights, never general agent/access/history/accounting write rights.
CREATE FUNCTION agent_factory.create_private(actor uuid, request uuid, config jsonb,
 candidate jsonb, purpose jsonb, consent jsonb) RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE old agent_factory.submissions%ROWTYPE; aid uuid := (candidate->>'id')::uuid;
BEGIN
 IF actor IS NULL OR actor='00000000-0000-0000-0000-000000000000'::uuid
 OR request IS NULL OR request='00000000-0000-0000-0000-000000000000'::uuid
 OR EXISTS(SELECT 1 FROM public.application_epoch) THEN
  RAISE EXCEPTION 'factory_scope_denied' USING ERRCODE='42501';
 END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('synthetic_factory:'||actor::text,0));
 SELECT * INTO old FROM agent_factory.submissions f WHERE f.creator_id=actor AND f.request_id=request;
 IF FOUND THEN
  IF old.configuration<>config THEN RAISE EXCEPTION 'factory_request_conflict' USING ERRCODE='40001'; END IF;
  IF NOT EXISTS(SELECT 1 FROM public.agent_access WHERE agent_id=old.agent_id AND principal_id=actor AND role='owner') THEN
   RAISE EXCEPTION 'factory_owner_revoked' USING ERRCODE='42501';
  END IF;
  RETURN old.agent_id;
 END IF;
 IF (SELECT count(*) FROM agent_factory.submissions WHERE creator_id=actor)>=100 THEN
  RAISE EXCEPTION 'factory_quota' USING ERRCODE='42501';
 END IF;
 IF (aid IS NOT NULL AND aid<>'00000000-0000-0000-0000-000000000000'::uuid
 AND config->>'request_id'=request::text AND config->>'autonomy'='disabled'
 AND config->'spending_limit_microusd'='0'::jsonb AND config->'private_consent'='true'::jsonb
 AND candidate->>'name'=config->>'name' AND candidate->>'lifecycle'='paused'
 AND candidate->'capabilities'='[]'::jsonb AND candidate->'memory'='[]'::jsonb
 AND candidate->'config'->'scheduler_enabled'='false'::jsonb
 AND candidate->'config'->'inference_cost_microusd'='0'::jsonb
 AND candidate->'config'->'operating_cost_microusd'='0'::jsonb
 AND candidate->'config'->'draft_cost_microusd'='0'::jsonb
 AND purpose->>'agent_id'=aid::text AND purpose->>'creator_id'=actor::text
 AND purpose->>'objective'=config->>'purpose' AND purpose->'revision'='1'::jsonb
 AND purpose->'public_summary'='null'::jsonb
 AND consent->'public'='false'::jsonb AND consent->'revoked'='true'::jsonb
 AND consent->'categories'='[]'::jsonb) IS NOT TRUE THEN
  RAISE EXCEPTION 'unsafe_factory_configuration' USING ERRCODE='42501';
 END IF;
 INSERT INTO public.agents(id,data) VALUES(aid,candidate);
 INSERT INTO public.agent_access(agent_id,principal_id,role,granted_by) VALUES(aid,actor,'owner',actor);
 INSERT INTO public.access_audit(agent_id,principal_id,role,approved_by) VALUES(aid,actor,'owner',actor);
 INSERT INTO public.lifecycle_transitions(agent_id,to_state,reason,actor) VALUES(aid,'paused','synthetic_factory_created','operator');
 INSERT INTO public.activity(id,agent_id,kind,code,simulated) VALUES(gen_random_uuid(),aid,'lifecycle','synthetic_factory_created_paused',true);
 INSERT INTO ecosystem_social.purposes(id,agent_id,creator_id,revision,body) VALUES((purpose->>'id')::uuid,aid,actor,1,purpose);
 INSERT INTO ecosystem_social.consents(request_id,agent_id,creator_id,revision,body) VALUES((consent->>'request_id')::uuid,aid,actor,1,consent);
 INSERT INTO agent_factory.submissions(creator_id,request_id,agent_id,configuration) VALUES(actor,request,aid,config);
 RETURN aid;
END;
$$;
CREATE FUNCTION agent_factory.owned(actor uuid) RETURNS TABLE(agent_id uuid,configuration jsonb)
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog AS $$
 SELECT f.agent_id,f.configuration FROM agent_factory.submissions f
 JOIN public.agent_access x ON x.agent_id=f.agent_id AND x.principal_id=f.creator_id AND x.role='owner'
 WHERE f.creator_id=actor ORDER BY f.recorded_at,f.agent_id LIMIT 100
$$;
CREATE FUNCTION agent_factory.managed(agent uuid) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog AS $$
 SELECT EXISTS(SELECT 1 FROM agent_factory.submissions WHERE agent_id=agent)
$$;
CREATE FUNCTION agent_factory.freeze_agent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM agent_factory.submissions WHERE agent_id=OLD.id) THEN
  RAISE EXCEPTION 'synthetic_factory_execution_unavailable' USING ERRCODE='42501';
 END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER synthetic_factory_agent_frozen BEFORE UPDATE OR DELETE ON public.agents
 FOR EACH ROW WHEN (OLD.id IS NOT NULL) EXECUTE FUNCTION agent_factory.freeze_agent();
REVOKE ALL ON FUNCTION agent_factory.create_private(uuid,uuid,jsonb,jsonb,jsonb,jsonb),
 agent_factory.owned(uuid),agent_factory.managed(uuid),agent_factory.freeze_agent() FROM PUBLIC;
-- Dedicated synthetic runtime: USAGE(agent_factory), EXECUTE(the three functions),
-- SELECT(application_epoch,_sqlx_migrations,agent_access) for existing auth/schema checks.
-- No public table writes, role memberships, migration authority or financial grants.
COMMIT;
