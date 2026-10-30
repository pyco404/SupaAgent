-- Separate approval authority from the requesting service. No roles, grants or approvals are provisioned.
CREATE SCHEMA epoch_guard;
REVOKE ALL ON SCHEMA epoch_guard FROM PUBLIC;
CREATE TABLE epoch_guard.authorities (
    login name PRIMARY KEY,
    principal_id uuid NOT NULL UNIQUE CHECK(principal_id <> '00000000-0000-0000-0000-000000000000')
);
CREATE TABLE epoch_guard.executors (
    login name PRIMARY KEY,
    principal_id uuid NOT NULL UNIQUE CHECK(principal_id <> '00000000-0000-0000-0000-000000000000'),
    role text NOT NULL CHECK(role='owner')
);
CREATE TABLE epoch_guard.approvals (
    id uuid PRIMARY KEY CHECK(id <> '00000000-0000-0000-0000-000000000000'),
    plan_text text NOT NULL,
    plan_sha256 text NOT NULL,
    issued_by uuid NOT NULL,
    issued_login name NOT NULL,
    issued_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    expires_at timestamptz NOT NULL
);
CREATE TABLE epoch_guard.revocations (
    approval_id uuid PRIMARY KEY REFERENCES epoch_guard.approvals(id),
    revoked_by uuid NOT NULL,
    revoked_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE epoch_guard.consumptions (
    approval_id uuid PRIMARY KEY REFERENCES epoch_guard.approvals(id),
    epoch_id uuid NOT NULL,
    revision bigint NOT NULL,
    principal_id uuid NOT NULL,
    executor_login name NOT NULL,
    operation text NOT NULL,
    plan_sha256 text NOT NULL,
    consumed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(epoch_id,revision)
);
REVOKE ALL ON ALL TABLES IN SCHEMA epoch_guard FROM PUBLIC;

CREATE FUNCTION epoch_guard.immutable() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN RAISE EXCEPTION 'epoch_security_records_are_immutable' USING ERRCODE='42501'; END;
$$;
CREATE TRIGGER authority_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON epoch_guard.authorities FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.immutable();
CREATE TRIGGER executor_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON epoch_guard.executors FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.immutable();
CREATE TRIGGER approval_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON epoch_guard.approvals FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.immutable();
CREATE TRIGGER revocation_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON epoch_guard.revocations FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.immutable();
CREATE TRIGGER consumption_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON epoch_guard.consumptions FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.immutable();

CREATE FUNCTION epoch_guard.validate_plan(raw text) RETURNS jsonb LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE p jsonb; keys text[]; expected_migrations jsonb; item jsonb;
BEGIN
    IF octet_length(raw)>32768 OR octet_length(raw)<2 THEN RAISE EXCEPTION 'invalid_epoch_plan'; END IF;
    p := raw::jsonb;
    SELECT array_agg(k ORDER BY k) INTO keys FROM jsonb_object_keys(p) k;
    IF keys IS DISTINCT FROM ARRAY['accounting_namespace','config_sha256','database','epoch_id','expected_revision','migrations_sha384','operation','principal_id','role','scope','version']::text[]
       OR p->'version' IS DISTINCT FROM '1'::jsonb
       OR p->>'database' IS DISTINCT FROM current_database()
       OR p->>'role' IS DISTINCT FROM 'owner'
       OR p->>'scope' IS DISTINCT FROM 'local_development_read_only'
       OR p->>'operation' IS NULL OR p->>'operation' NOT IN ('activate_read_only','rollback')
       OR jsonb_typeof(p->'expected_revision') IS DISTINCT FROM 'number'
       OR p->>'expected_revision' !~ '^(0|[1-9][0-9]*)$'
       OR (p->>'expected_revision')::bigint < 0
       OR p->>'epoch_id' IS NULL OR (p->>'epoch_id')::uuid = '00000000-0000-0000-0000-000000000000'
       OR p->>'principal_id' IS NULL OR (p->>'principal_id')::uuid = '00000000-0000-0000-0000-000000000000'
       OR p->>'accounting_namespace' IS NULL OR (p->>'accounting_namespace')::uuid = '00000000-0000-0000-0000-000000000000'
       OR jsonb_typeof(p->'config_sha256') IS DISTINCT FROM 'object' THEN
       RAISE EXCEPTION 'invalid_epoch_plan';
    END IF;
    SELECT array_agg(k ORDER BY k) INTO keys FROM jsonb_object_keys(p->'config_sha256') k;
    IF keys IS DISTINCT FROM ARRAY['api','frontend','postgres']::text[] THEN RAISE EXCEPTION 'invalid_configuration_binding'; END IF;
    FOR item IN SELECT value FROM jsonb_each(p->'config_sha256') LOOP
        IF jsonb_typeof(item) IS DISTINCT FROM 'string' OR item#>>'{}' !~ '^[0-9a-f]{64}$' THEN RAISE EXCEPTION 'invalid_configuration_binding'; END IF;
    END LOOP;
    SELECT jsonb_object_agg(version::text,encode(checksum,'hex')) INTO expected_migrations FROM public._sqlx_migrations WHERE success;
    IF p->'migrations_sha384' IS DISTINCT FROM expected_migrations OR NOT (expected_migrations ? '10') THEN
        RAISE EXCEPTION 'migration_binding_mismatch';
    END IF;
    RETURN p;
END;
$$;

CREATE FUNCTION epoch_guard.approve(id uuid, raw text, expires timestamptz) RETURNS text
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE issuer uuid; p jsonb; digest text;
BEGIN
    SELECT principal_id INTO issuer FROM epoch_guard.authorities WHERE login=session_user;
    IF issuer IS NULL OR expires<=clock_timestamp() OR expires>clock_timestamp()+interval '1 hour' THEN
        RAISE EXCEPTION 'independent_epoch_authority_required' USING ERRCODE='42501';
    END IF;
    p := epoch_guard.validate_plan(raw);
    -- Authority must be different from the executor login, even if privileges were accidentally combined.
    IF EXISTS(SELECT 1 FROM epoch_guard.executors WHERE login=session_user) THEN
        RAISE EXCEPTION 'authority_executor_separation_required' USING ERRCODE='42501';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM epoch_guard.executors WHERE principal_id=(p->>'principal_id')::uuid AND role=p->>'role') THEN
        RAISE EXCEPTION 'approved_executor_binding_required';
    END IF;
    digest := encode(sha256(convert_to(raw,'UTF8')),'hex');
    INSERT INTO epoch_guard.approvals(id,plan_text,plan_sha256,issued_by,issued_login,expires_at)
        VALUES(id,raw,digest,issuer,session_user,expires);
    RETURN digest;
END;
$$;
CREATE FUNCTION epoch_guard.revoke_approval(id uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE issuer uuid;
BEGIN
    SELECT principal_id INTO issuer FROM epoch_guard.authorities WHERE login=session_user;
    IF issuer IS NULL THEN RAISE EXCEPTION 'independent_epoch_authority_required' USING ERRCODE='42501'; END IF;
    INSERT INTO epoch_guard.revocations(approval_id,revoked_by) VALUES(id,issuer);
END;
$$;

CREATE FUNCTION epoch_guard.execute_transition(approval uuid, actor uuid, actor_role text, epoch uuid, expected bigint, operation text, raw text)
RETURNS jsonb LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE consent epoch_guard.approvals%ROWTYPE; e public.application_epoch%ROWTYPE; p jsonb; digest text; next_state text;
BEGIN
    IF current_setting('default_transaction_read_only')<>'on'
       OR NOT EXISTS(SELECT 1 FROM epoch_guard.executors WHERE login=session_user AND principal_id=actor AND role=actor_role)
       OR EXISTS(SELECT 1 FROM epoch_guard.authorities WHERE login=session_user) THEN
        RAISE EXCEPTION 'scoped_epoch_executor_required' USING ERRCODE='42501';
    END IF;
    SELECT * INTO STRICT consent FROM epoch_guard.approvals WHERE id=approval FOR UPDATE;
    IF consent.expires_at<=clock_timestamp() OR EXISTS(SELECT 1 FROM epoch_guard.revocations WHERE approval_id=approval)
       OR EXISTS(SELECT 1 FROM epoch_guard.consumptions WHERE approval_id=approval) THEN
        RAISE EXCEPTION 'epoch_approval_expired_revoked_or_consumed';
    END IF;
    digest := encode(sha256(convert_to(raw,'UTF8')),'hex');
    IF raw IS DISTINCT FROM consent.plan_text OR digest IS DISTINCT FROM consent.plan_sha256 THEN
        RAISE EXCEPTION 'trusted_epoch_plan_mismatch';
    END IF;
    p := epoch_guard.validate_plan(raw);
    IF (p->>'epoch_id')::uuid IS DISTINCT FROM epoch OR (p->>'principal_id')::uuid IS DISTINCT FROM actor
       OR p->>'role' IS DISTINCT FROM actor_role OR (p->>'expected_revision')::bigint IS DISTINCT FROM expected
       OR p->>'operation' IS DISTINCT FROM operation THEN RAISE EXCEPTION 'epoch_approval_scope_mismatch'; END IF;
    SELECT * INTO STRICT e FROM public.application_epoch WHERE id=epoch FOR UPDATE;
    next_state := CASE WHEN operation='activate_read_only' AND e.state='prepared' THEN 'active_read_only'
                       WHEN operation='rollback' AND e.state='active_read_only' THEN 'prepared' ELSE NULL END;
    IF next_state IS NULL OR e.revision<>expected OR e.accounting_namespace<>(p->>'accounting_namespace')::uuid
       OR e.kind<>'development' OR e.historical_database<>'unrecovered' THEN RAISE EXCEPTION 'stale_epoch_or_namespace'; END IF;
    INSERT INTO epoch_guard.consumptions(approval_id,epoch_id,revision,principal_id,executor_login,operation,plan_sha256)
        VALUES(approval,epoch,expected+1,actor,session_user,operation,digest);
    INSERT INTO public.epoch_transitions(approval_id,epoch_id,revision,operation,approved_by,plan_sha256,from_state,to_state)
        VALUES(approval,epoch,expected+1,operation,actor,digest,e.state,next_state);
    UPDATE public.application_epoch SET state=next_state,revision=expected+1,approval_id=approval WHERE id=epoch;
    RETURN (SELECT to_jsonb(x)-'singleton'-'created_at' FROM public.application_epoch x WHERE id=epoch);
END;
$$;

-- Defense against direct SQL and the old caller-controlled-DML transition, including accidental DML grants.
CREATE FUNCTION epoch_guard.protect_epoch() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
BEGIN
    IF TG_OP IN ('DELETE','TRUNCATE') THEN RAISE EXCEPTION 'epoch_deletion_forbidden'; END IF;
    IF TG_OP='INSERT' THEN
        IF NEW.state<>'prepared' OR NEW.revision<>0 OR NEW.approval_id IS NOT NULL THEN RAISE EXCEPTION 'epoch_must_begin_prepared'; END IF;
        RETURN NEW;
    END IF;
    IF (NEW.id,NEW.accounting_namespace,NEW.kind,NEW.historical_database) IS DISTINCT FROM (OLD.id,OLD.accounting_namespace,OLD.kind,OLD.historical_database) THEN
        RAISE EXCEPTION 'epoch_identity_is_immutable';
    END IF;
    IF (NEW.state,NEW.revision,NEW.approval_id) IS DISTINCT FROM (OLD.state,OLD.revision,OLD.approval_id) THEN
        IF NEW.revision<>OLD.revision+1 OR NOT EXISTS(
            SELECT 1 FROM epoch_guard.consumptions c JOIN epoch_guard.approvals a ON a.id=c.approval_id
            JOIN public.epoch_transitions t ON t.approval_id=c.approval_id
            WHERE c.approval_id=NEW.approval_id AND c.epoch_id=NEW.id AND c.revision=NEW.revision
              AND t.from_state=OLD.state AND t.to_state=NEW.state AND t.revision=NEW.revision
              AND t.plan_sha256=c.plan_sha256 AND c.plan_sha256=a.plan_sha256
        ) THEN RAISE EXCEPTION 'trusted_consumed_approval_required'; END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER epoch_row_guard BEFORE INSERT OR UPDATE OR DELETE ON public.application_epoch FOR EACH ROW EXECUTE FUNCTION epoch_guard.protect_epoch();
CREATE TRIGGER epoch_truncate_guard BEFORE TRUNCATE ON public.application_epoch FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.protect_epoch();
CREATE FUNCTION epoch_guard.protect_audit() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM epoch_guard.consumptions c JOIN epoch_guard.approvals a ON a.id=c.approval_id
        JOIN public.application_epoch e ON e.id=c.epoch_id
        WHERE c.approval_id=NEW.approval_id AND c.epoch_id=NEW.epoch_id AND c.revision=NEW.revision
          AND c.principal_id=NEW.approved_by AND c.operation=NEW.operation AND c.plan_sha256=NEW.plan_sha256
          AND c.plan_sha256=a.plan_sha256 AND e.revision+1=NEW.revision AND e.state=NEW.from_state
          AND ((NEW.operation='activate_read_only' AND NEW.to_state='active_read_only') OR (NEW.operation='rollback' AND NEW.to_state='prepared'))
    ) THEN RAISE EXCEPTION 'untrusted_transition_audit_forbidden'; END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER epoch_audit_guard BEFORE INSERT ON public.epoch_transitions FOR EACH ROW EXECUTE FUNCTION epoch_guard.protect_audit();

CREATE FUNCTION epoch_guard.verify_active(epoch uuid, approval uuid) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog AS $$
    SELECT EXISTS(SELECT 1 FROM public.application_epoch e JOIN epoch_guard.consumptions c ON c.approval_id=e.approval_id
        JOIN epoch_guard.approvals a ON a.id=c.approval_id JOIN public.epoch_transitions t ON t.approval_id=c.approval_id
        WHERE e.id=epoch AND e.approval_id=approval AND e.state='active_read_only' AND e.revision=c.revision
          AND c.epoch_id=e.id AND c.operation='activate_read_only' AND t.operation=c.operation AND t.to_state=e.state
          AND t.plan_sha256=a.plan_sha256 AND c.plan_sha256=a.plan_sha256
          AND a.plan_sha256=encode(sha256(convert_to(a.plan_text,'UTF8')),'hex')
          AND epoch_guard.validate_plan(a.plan_text)->>'principal_id'=c.principal_id::text
          AND a.plan_text::jsonb->>'epoch_id'=e.id::text
          AND a.plan_text::jsonb->>'accounting_namespace'=e.accounting_namespace::text
          AND (a.plan_text::jsonb->>'expected_revision')::bigint+1=e.revision
          AND a.plan_text::jsonb->>'operation'=c.operation
          AND NOT EXISTS(SELECT 1 FROM epoch_guard.revocations r WHERE r.approval_id=a.id));
$$;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA epoch_guard FROM PUBLIC;
-- Read-only verification discloses only a boolean; approval issuance/execution receive no default grants.
GRANT USAGE ON SCHEMA epoch_guard TO PUBLIC;
GRANT EXECUTE ON FUNCTION epoch_guard.verify_active(uuid,uuid) TO PUBLIC;
