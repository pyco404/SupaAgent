-- Installs no identity, approval, credential, grant or activation. Prepared remains the default.
CREATE TABLE epoch_guard.operational_principals (
 credential_id uuid PRIMARY KEY, principal_id uuid NOT NULL,
 login name NOT NULL UNIQUE, verified_by uuid NOT NULL, verified_at timestamptz NOT NULL,
 expires_at timestamptz NOT NULL, CHECK(principal_id<>verified_by)
);
CREATE TABLE epoch_guard.credential_revocations (
 credential_id uuid PRIMARY KEY REFERENCES epoch_guard.operational_principals,
 revoked_by uuid NOT NULL, revoked_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE epoch_guard.operational_approvals (
 id uuid PRIMARY KEY, plan jsonb NOT NULL, issued_by uuid NOT NULL,
 issued_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE epoch_guard.operational_revocations (
 approval_id uuid PRIMARY KEY REFERENCES epoch_guard.operational_approvals,
 revoked_by uuid NOT NULL, revoked_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE epoch_guard.operational_claims (
 approval_id uuid PRIMARY KEY REFERENCES epoch_guard.operational_approvals,
 secret_hash bytea NOT NULL, login name NOT NULL, server_start timestamptz NOT NULL,
 claimed_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE epoch_guard.operational_operations (
 approval_id uuid NOT NULL REFERENCES epoch_guard.operational_claims,
 transaction_id bigint NOT NULL, occurred_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(approval_id,transaction_id)
);
ALTER TABLE cycles ADD COLUMN epoch_permit_id uuid REFERENCES epoch_guard.operational_claims;
REVOKE ALL ON ALL TABLES IN SCHEMA epoch_guard FROM PUBLIC;
DO $$ DECLARE t text; BEGIN
 FOREACH t IN ARRAY ARRAY['operational_principals','credential_revocations','operational_approvals','operational_revocations','operational_claims','operational_operations'] LOOP
  EXECUTE format('CREATE TRIGGER immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON epoch_guard.%I FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.immutable()',t);
 END LOOP;
END $$;

CREATE FUNCTION epoch_guard.operational_plan(p jsonb) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE e public.application_epoch%ROWTYPE; keys text[]; who epoch_guard.operational_principals%ROWTYPE;
BEGIN
 SELECT array_agg(k ORDER BY k) INTO keys FROM jsonb_object_keys(p) k;
 IF keys IS DISTINCT FROM ARRAY['accounting_namespace','agent_id','budget_microusd','credential_id','database','epoch_id','expected_revision','expires_at','max_cycles','max_transactions','principal_id','scope','seed_microusd','version']::text[]
 OR p->'version' IS DISTINCT FROM '1'::jsonb
 OR p->>'scope' NOT IN ('agent_creation','ownership_grant','offline_control','offline_execution')
 OR p->>'database' IS DISTINCT FROM current_database()
 OR (p->>'expires_at')::timestamptz<=clock_timestamp()
 OR (p->>'expires_at')::timestamptz>clock_timestamp()+interval '1 hour'
 OR (p->>'max_transactions')::int NOT BETWEEN 1 AND 10000
 OR (p->>'max_cycles')::int NOT BETWEEN 0 AND 100
 OR (p->>'budget_microusd')::bigint NOT BETWEEN 0 AND 1000000
 OR (p->>'seed_microusd')::bigint NOT BETWEEN 0 AND 1000000
 OR (p->>'agent_id')::uuid='00000000-0000-0000-0000-000000000000'
 THEN RAISE EXCEPTION 'invalid_operational_scope' USING ERRCODE='42501'; END IF;
 -- JSON nulls must not evade SQL three-valued checks.
 IF EXISTS(SELECT 1 FROM jsonb_each(p) WHERE value='null'::jsonb) THEN RAISE EXCEPTION 'null_operational_binding'; END IF;
 SELECT * INTO STRICT e FROM public.application_epoch;
 IF e.id IS DISTINCT FROM (p->>'epoch_id')::uuid OR e.accounting_namespace IS DISTINCT FROM (p->>'accounting_namespace')::uuid
 OR e.revision IS DISTINCT FROM (p->>'expected_revision')::bigint
 OR e.kind<>'development' OR e.historical_database<>'unrecovered' OR e.state NOT IN ('prepared','active_read_only')
 THEN RAISE EXCEPTION 'wrong_operational_epoch'; END IF;
 SELECT * INTO STRICT who FROM epoch_guard.operational_principals WHERE credential_id=(p->>'credential_id')::uuid;
 IF who.principal_id IS DISTINCT FROM (p->>'principal_id')::uuid OR who.verified_at>clock_timestamp()
 OR who.expires_at<(p->>'expires_at')::timestamptz
 OR NOT EXISTS(SELECT 1 FROM epoch_guard.authorities WHERE principal_id=who.verified_by)
 OR NOT EXISTS(SELECT 1 FROM epoch_guard.executors WHERE login=who.login AND principal_id=who.principal_id AND role='owner')
 OR EXISTS(SELECT 1 FROM epoch_guard.credential_revocations WHERE credential_id=who.credential_id)
 THEN RAISE EXCEPTION 'verified_operator_required'; END IF;
 RETURN p;
END $$;

CREATE FUNCTION epoch_guard.approve_operation(id uuid,p jsonb) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE issuer uuid;
BEGIN
 SELECT principal_id INTO STRICT issuer FROM epoch_guard.authorities WHERE login=session_user;
 IF EXISTS(SELECT 1 FROM epoch_guard.executors WHERE login=session_user)
 OR issuer=(p->>'principal_id')::uuid OR id='00000000-0000-0000-0000-000000000000'
 THEN RAISE EXCEPTION 'independent_authority_required'; END IF;
 PERFORM epoch_guard.operational_plan(p);
 INSERT INTO epoch_guard.operational_approvals VALUES(id,p,issuer,clock_timestamp());
END $$;
CREATE FUNCTION epoch_guard.revoke_operation(id uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE issuer uuid;
BEGIN
 SELECT principal_id INTO STRICT issuer FROM epoch_guard.authorities WHERE login=session_user;
 PERFORM pg_advisory_xact_lock(hashtextextended(id::text,812));
 INSERT INTO epoch_guard.operational_revocations(approval_id,revoked_by) VALUES(id,issuer);
END $$;
CREATE FUNCTION epoch_guard.revoke_credential(id uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE issuer uuid; permit uuid;
BEGIN
 SELECT principal_id INTO STRICT issuer FROM epoch_guard.authorities WHERE login=session_user;
 -- Stable ordering; revocation serializes with every affected operation.
 FOR permit IN SELECT a.id FROM epoch_guard.operational_approvals a WHERE a.plan->>'credential_id'=revoke_credential.id::text ORDER BY a.id LOOP
  PERFORM pg_advisory_xact_lock(hashtextextended(permit::text,812));
 END LOOP;
 INSERT INTO epoch_guard.credential_revocations(credential_id,revoked_by) VALUES(id,issuer);
END $$;
CREATE FUNCTION epoch_guard.claim_operation(id uuid,epoch uuid,namespace uuid) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE p jsonb; secret text;
BEGIN
 SELECT plan INTO STRICT p FROM epoch_guard.operational_approvals WHERE operational_approvals.id=claim_operation.id FOR UPDATE;
 PERFORM epoch_guard.operational_plan(p);
 IF (p->>'epoch_id')::uuid IS DISTINCT FROM epoch OR (p->>'accounting_namespace')::uuid IS DISTINCT FROM namespace
 OR NOT EXISTS(SELECT 1 FROM epoch_guard.operational_principals WHERE credential_id=(p->>'credential_id')::uuid AND login=session_user)
 OR EXISTS(SELECT 1 FROM epoch_guard.operational_revocations WHERE approval_id=id)
 OR EXISTS(SELECT 1 FROM pg_roles WHERE rolname=session_user AND (rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls))
 OR EXISTS(SELECT 1 FROM epoch_guard.authorities WHERE login=session_user)
 OR has_schema_privilege(session_user,'public','CREATE')
 OR EXISTS(SELECT 1 FROM pg_tables WHERE schemaname='epoch_guard' AND has_table_privilege(session_user,format('%I.%I',schemaname,tablename),'INSERT,UPDATE,DELETE,TRUNCATE'))
 OR current_setting('listen_addresses')<>''
 OR current_setting('default_transaction_read_only')<>'on'
 THEN RAISE EXCEPTION 'scoped_operational_login_required'; END IF;
 secret:=gen_random_uuid()::text||gen_random_uuid()::text;
 INSERT INTO epoch_guard.operational_claims VALUES(id,sha256(convert_to(secret,'UTF8')),session_user,pg_postmaster_start_time(),clock_timestamp());
 RETURN p||jsonb_build_object('approval_id',id,'secret',secret);
END $$;

CREATE FUNCTION epoch_guard.current_operation() RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE permit uuid; p jsonb; c epoch_guard.operational_claims%ROWTYPE;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' THEN RAISE EXCEPTION 'operational_isolation_required'; END IF;
 permit:=nullif(current_setting('supa.operation_id',true),'')::uuid;
 IF permit IS NULL THEN RAISE EXCEPTION 'operational_authorization_required' USING ERRCODE='42501'; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended(permit::text,812));
 SELECT * INTO STRICT c FROM epoch_guard.operational_claims WHERE approval_id=permit;
 SELECT plan INTO STRICT p FROM epoch_guard.operational_approvals WHERE operational_approvals.id=permit;
 IF c.login<>session_user OR c.server_start<>pg_postmaster_start_time()
 OR c.secret_hash IS DISTINCT FROM sha256(convert_to(current_setting('supa.operation_secret',true),'UTF8'))
 OR EXISTS(SELECT 1 FROM epoch_guard.operational_revocations WHERE approval_id=permit)
 THEN RAISE EXCEPTION 'operational_authorization_invalid' USING ERRCODE='42501'; END IF;
 PERFORM epoch_guard.operational_plan(p);
 RETURN p||jsonb_build_object('approval_id',permit);
END $$;

CREATE FUNCTION epoch_guard.guard_operation() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE p jsonb; resource uuid; rowdata jsonb; c public.cycles%ROWTYPE; n bigint; scope text; consent_expiry timestamptz;
BEGIN
 IF NOT EXISTS(SELECT 1 FROM public.application_epoch) THEN RETURN coalesce(NEW,OLD); END IF;
 IF TG_OP IN ('DELETE','TRUNCATE') THEN RAISE EXCEPTION 'operational_deletion_forbidden'; END IF;
 p:=epoch_guard.current_operation(); scope:=p->>'scope'; rowdata:=to_jsonb(NEW);
 IF TG_OP='UPDATE' AND (rowdata->'id',rowdata->'agent_id',rowdata->'cycle_id',rowdata->'epoch_permit_id') IS DISTINCT FROM (to_jsonb(OLD)->'id',to_jsonb(OLD)->'agent_id',to_jsonb(OLD)->'cycle_id',to_jsonb(OLD)->'epoch_permit_id') THEN RAISE EXCEPTION 'operational_identity_is_immutable'; END IF;
 IF TG_OP<>'INSERT' AND TG_TABLE_NAME IN ('simulated_ledger','activity','access_audit','lifecycle_transitions','goal_evaluations','schedule_commands') THEN RAISE EXCEPTION 'operational_audit_is_append_only'; END IF;

 IF TG_TABLE_NAME='worker_health' THEN
  IF scope NOT IN ('offline_control','offline_execution') THEN RAISE EXCEPTION 'worker_scope_required'; END IF;
  IF TG_OP='INSERT' THEN NEW.epoch_permit_id:=(p->>'approval_id')::uuid;
  ELSIF NEW.epoch_permit_id IS DISTINCT FROM OLD.epoch_permit_id OR NEW.epoch_permit_id IS DISTINCT FROM (p->>'approval_id')::uuid THEN RAISE EXCEPTION 'worker_permit_mismatch'; END IF;
 ELSE
  resource:=CASE WHEN TG_TABLE_NAME='agents' THEN (rowdata->>'id')::uuid ELSE (rowdata->>'agent_id')::uuid END;
  IF TG_TABLE_NAME='actions' THEN SELECT agent_id INTO STRICT resource FROM public.cycles WHERE id=NEW.cycle_id; END IF;
  IF resource IS DISTINCT FROM (p->>'agent_id')::uuid THEN RAISE EXCEPTION 'operational_agent_mismatch'; END IF;
  IF scope='agent_creation' THEN
   IF TG_TABLE_NAME NOT IN ('agents','agent_access','access_audit','lifecycle_transitions','activity') THEN RAISE EXCEPTION 'creation_scope_only'; END IF;
   IF TG_TABLE_NAME='agents' AND (rowdata->'data'->>'lifecycle' IS DISTINCT FROM 'created' OR (rowdata->'data'->'config'->>'scheduler_enabled')::boolean IS DISTINCT FROM false) THEN RAISE EXCEPTION 'inactive_creation_only'; END IF;
   IF TG_TABLE_NAME IN ('agent_access','access_audit') AND (rowdata->>'principal_id' IS DISTINCT FROM p->>'principal_id' OR rowdata->>'role'<>'owner' OR coalesce(rowdata->>'granted_by',rowdata->>'approved_by') IS DISTINCT FROM p->>'principal_id') THEN RAISE EXCEPTION 'verified_owner_binding_required'; END IF;
  ELSIF scope='ownership_grant' THEN
   -- Initial adoption only; no delegation, implicit ownership or overwrite.
   IF TG_TABLE_NAME NOT IN ('agent_access','access_audit') OR TG_OP<>'INSERT'
    OR rowdata->>'principal_id' IS DISTINCT FROM p->>'principal_id' OR rowdata->>'role'<>'owner'
    OR coalesce(rowdata->>'granted_by',rowdata->>'approved_by') IS DISTINCT FROM p->>'principal_id'
   THEN RAISE EXCEPTION 'explicit_initial_owner_only'; END IF;
  ELSIF scope IN ('offline_control','offline_execution') THEN
   IF NOT EXISTS(SELECT 1 FROM public.agent_access WHERE agent_id=resource AND principal_id=(p->>'principal_id')::uuid AND role='owner') THEN RAISE EXCEPTION 'operational_owner_required'; END IF;
   IF TG_TABLE_NAME NOT IN ('agents','cycles','actions','activity','simulated_ledger','inference_calls','goal_evaluations','lifecycle_transitions','schedule_commands','agent_schedules','schedule_authorizations') THEN RAISE EXCEPTION 'offline_scope_only'; END IF;
   IF scope='offline_execution' AND TG_TABLE_NAME IN ('schedule_commands','agent_schedules','schedule_authorizations','lifecycle_transitions') AND TG_OP='INSERT' THEN
    -- Runtime may pause at limits, but cannot mint a new scheduling consent.
    IF TG_TABLE_NAME<>'lifecycle_transitions' THEN RAISE EXCEPTION 'separate_activation_required'; END IF;
   END IF;
   IF scope='offline_execution' AND TG_OP='UPDATE' THEN
    IF TG_TABLE_NAME='agent_schedules' AND (rowdata->'policy' IS DISTINCT FROM to_jsonb(OLD)->'policy' OR rowdata->'authorization_id' IS DISTINCT FROM to_jsonb(OLD)->'authorization_id' OR rowdata->'expires_at' IS DISTINCT FROM to_jsonb(OLD)->'expires_at' OR ((rowdata->>'enabled')::boolean AND NOT (to_jsonb(OLD)->>'enabled')::boolean)) THEN RAISE EXCEPTION 'execution_cannot_renew_consent'; END IF;
    IF TG_TABLE_NAME='schedule_authorizations' AND (rowdata->'policy' IS DISTINCT FROM to_jsonb(OLD)->'policy' OR rowdata->'expires_at' IS DISTINCT FROM to_jsonb(OLD)->'expires_at' OR rowdata->'approved_by' IS DISTINCT FROM to_jsonb(OLD)->'approved_by') THEN RAISE EXCEPTION 'execution_cannot_renew_consent'; END IF;
    IF TG_TABLE_NAME='agents' AND rowdata->'data'->>'lifecycle' IS DISTINCT FROM to_jsonb(OLD)->'data'->>'lifecycle' AND rowdata->'data'->>'lifecycle' NOT IN ('paused','suspended') THEN RAISE EXCEPTION 'execution_cannot_activate'; END IF;
   END IF;
   IF TG_TABLE_NAME IN ('schedule_commands','schedule_authorizations') AND rowdata->>'approved_by' IS DISTINCT FROM p->>'principal_id' THEN RAISE EXCEPTION 'verified_offline_consent_required'; END IF;
   IF TG_TABLE_NAME IN ('agent_schedules','schedule_authorizations') AND rowdata->'policy' IS NOT NULL THEN
    IF (rowdata->'policy'->>'max_cycles')::int>(p->>'max_cycles')::int
    OR (rowdata->'policy'->>'simulated_budget_microusd')::bigint>(p->>'budget_microusd')::bigint
    OR (rowdata->>'expires_at')::timestamptz>(p->>'expires_at')::timestamptz
    THEN RAISE EXCEPTION 'finite_activation_ceiling'; END IF;
   END IF;
   IF TG_TABLE_NAME='cycles' THEN
    IF TG_OP='INSERT' THEN
     IF scope<>'offline_execution' THEN RAISE EXCEPTION 'separate_execution_required'; END IF;
     SELECT count(*) INTO n FROM public.cycles WHERE epoch_permit_id=(p->>'approval_id')::uuid;
     IF n>=(p->>'max_cycles')::int THEN RAISE EXCEPTION 'epoch_cycle_ceiling'; END IF;
     SELECT expires_at INTO STRICT consent_expiry FROM public.schedule_authorizations WHERE id=NEW.authorization_id AND agent_id=resource AND approved_by=(p->>'principal_id')::uuid AND closed_at IS NULL;
     NEW.epoch_permit_id:=(p->>'approval_id')::uuid;
     NEW.lease_until:=least(NEW.lease_until,(p->>'expires_at')::timestamptz,consent_expiry);
     IF NEW.lease_until<=clock_timestamp() THEN RAISE EXCEPTION 'finite_consent_expired'; END IF;
    ELSIF scope='offline_execution' AND NEW.epoch_permit_id IS DISTINCT FROM (p->>'approval_id')::uuid THEN RAISE EXCEPTION 'stale_epoch_cycle';
    ELSIF scope='offline_control' AND NEW.status NOT IN ('interrupted','failed','running') THEN RAISE EXCEPTION 'control_cannot_complete_cycles'; END IF;
   END IF;
   IF TG_TABLE_NAME='inference_calls' AND (rowdata->>'budget_id' IS NOT NULL OR rowdata->'policy'->>'live' IS DISTINCT FROM 'false') THEN RAISE EXCEPTION 'paid_inference_disabled'; END IF;
   IF TG_TABLE_NAME='simulated_ledger' THEN
    IF NEW.kind='seed' AND scope='offline_control' THEN
     SELECT coalesce(sum(amount_microusd),0) INTO n FROM public.simulated_ledger WHERE agent_id=resource AND kind='seed';
     IF n+NEW.amount_microusd>(p->>'seed_microusd')::bigint THEN RAISE EXCEPTION 'simulated_seed_ceiling'; END IF;
    ELSIF NEW.kind NOT IN ('inference','operating') OR scope<>'offline_execution' THEN RAISE EXCEPTION 'financial_actions_disabled'; END IF;
    IF NEW.amount_microusd<0 THEN
     SELECT coalesce(-sum(l.amount_microusd),0) INTO n FROM public.simulated_ledger l JOIN public.cycles x ON x.id=l.cycle_id WHERE x.epoch_permit_id=(p->>'approval_id')::uuid AND l.amount_microusd<0;
     IF n-NEW.amount_microusd>(p->>'budget_microusd')::bigint THEN RAISE EXCEPTION 'epoch_simulated_budget'; END IF;
    END IF;
   END IF;
   IF TG_TABLE_NAME IN ('actions','inference_calls','goal_evaluations','simulated_ledger') AND rowdata->>'cycle_id' IS NOT NULL THEN
    SELECT * INTO STRICT c FROM public.cycles WHERE id=(rowdata->>'cycle_id')::uuid;
    IF scope='offline_control' AND TG_OP='UPDATE' AND c.status='interrupted' AND c.lease_until<=clock_timestamp()
     AND ((TG_TABLE_NAME='actions' AND rowdata->>'status'='failed') OR (TG_TABLE_NAME='inference_calls' AND rowdata->>'status'='unknown')) THEN NULL;
    ELSIF scope<>'offline_execution' OR c.epoch_permit_id IS DISTINCT FROM (p->>'approval_id')::uuid OR c.lease_until<=clock_timestamp() THEN RAISE EXCEPTION 'stale_epoch_lease'; END IF;
   END IF;
  ELSE RAISE EXCEPTION 'operational_scope_denied'; END IF;
  IF TG_TABLE_NAME='agents' THEN
   IF NEW.data->'capabilities' IS NULL OR EXISTS(SELECT 1 FROM jsonb_array_elements_text(NEW.data->'capabilities') cap WHERE cap NOT IN ('read_memory','remember','evaluate_goal')) THEN RAISE EXCEPTION 'offline_capabilities_only'; END IF;
  END IF;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM epoch_guard.operational_operations WHERE approval_id=(p->>'approval_id')::uuid AND transaction_id=txid_current()) THEN
  SELECT count(*) INTO n FROM epoch_guard.operational_operations WHERE approval_id=(p->>'approval_id')::uuid;
  IF n>=(p->>'max_transactions')::int THEN RAISE EXCEPTION 'epoch_transaction_ceiling'; END IF;
  INSERT INTO epoch_guard.operational_operations(approval_id,transaction_id) VALUES((p->>'approval_id')::uuid,txid_current());
 END IF;
 RETURN NEW;
END $$;
ALTER TABLE worker_health ADD COLUMN epoch_permit_id uuid REFERENCES epoch_guard.operational_claims;
-- Recheck at commit: no transaction may hold a pre-expiry check and commit later.
CREATE FUNCTION epoch_guard.commit_operation() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE resource uuid; consistent boolean;
BEGIN
 IF EXISTS(SELECT 1 FROM public.application_epoch) THEN
  PERFORM epoch_guard.current_operation();
  IF TG_TABLE_NAME IN ('agents','simulated_ledger') THEN
   resource:=CASE WHEN TG_TABLE_NAME='agents' THEN (to_jsonb(NEW)->>'id')::uuid ELSE (to_jsonb(NEW)->>'agent_id')::uuid END;
   SELECT a.simulated_balance_microusd=coalesce((SELECT sum(amount_microusd) FROM public.simulated_ledger WHERE agent_id=resource),0) INTO consistent FROM public.agents a WHERE a.id=resource;
   IF consistent IS DISTINCT FROM true THEN RAISE EXCEPTION 'epoch_accounting_invariant'; END IF;
  END IF;
 END IF;
 RETURN NULL;
END $$;
DO $$ DECLARE t text; BEGIN
 FOR t IN SELECT tablename FROM pg_tables WHERE schemaname='public' AND tablename NOT IN ('_sqlx_migrations','application_epoch','epoch_transitions') LOOP
  EXECUTE format('CREATE TRIGGER operational_write BEFORE INSERT OR UPDATE OR DELETE ON public.%I FOR EACH ROW EXECUTE FUNCTION epoch_guard.guard_operation()',t);
  EXECUTE format('CREATE TRIGGER operational_truncate BEFORE TRUNCATE ON public.%I FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.guard_operation()',t);
  EXECUTE format('CREATE CONSTRAINT TRIGGER operational_commit AFTER INSERT OR UPDATE ON public.%I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION epoch_guard.commit_operation()',t);
 END LOOP;
END $$;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA epoch_guard FROM PUBLIC;
GRANT EXECUTE ON FUNCTION epoch_guard.verify_active(uuid,uuid) TO PUBLIC;
-- Self-checking admission/validation; authority functions receive no default privileges.
GRANT EXECUTE ON FUNCTION epoch_guard.claim_operation(uuid,uuid,uuid),epoch_guard.current_operation() TO PUBLIC;

CREATE FUNCTION epoch_guard.operation_live(id uuid) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE p jsonb;
BEGIN
 SELECT a.plan INTO STRICT p FROM epoch_guard.operational_approvals a JOIN epoch_guard.operational_claims c ON c.approval_id=a.id WHERE a.id=operation_live.id AND c.server_start=pg_postmaster_start_time() AND NOT EXISTS(SELECT 1 FROM epoch_guard.operational_revocations r WHERE r.approval_id=a.id);
 PERFORM epoch_guard.operational_plan(p);
 RETURN true;
EXCEPTION WHEN OTHERS THEN RETURN false;
END $$;
REVOKE ALL ON FUNCTION epoch_guard.operation_live(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION epoch_guard.operation_live(uuid) TO PUBLIC;
