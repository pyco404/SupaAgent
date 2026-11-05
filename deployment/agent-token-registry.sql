-- Opt-in disposable-only extension; NOT one of the 13 automatic migrations.
-- A separately authorized trusted verifier must validate the existing Rust domain
-- aggregate, current qualification, confirmed chain evidence and (for successors)
-- explicit approval plus independently verified public disclosure before attesting.
-- No HTTP/model input may populate receipts. No default runtime or verifier grants.
BEGIN;
CREATE SCHEMA agent_tokens;
REVOKE ALL ON SCHEMA agent_tokens FROM PUBLIC;
CREATE TABLE agent_tokens.receipts (
 id uuid PRIMARY KEY CHECK(id<>'00000000-0000-0000-0000-000000000000'),
 agent_id uuid NOT NULL REFERENCES public.agents(id), creator_id uuid NOT NULL,
 kind text NOT NULL CHECK(kind IN ('original','successor')),
 original_id uuid,
 purpose_id uuid NOT NULL REFERENCES ecosystem_social.purposes(id),
 qualification_id uuid NOT NULL REFERENCES ecosystem_social.owner_reviews(id),
 network text NOT NULL CHECK(network IN ('mainnet','devnet')),
 mint text NOT NULL CHECK(mint ~ '^[1-9A-HJ-NP-Za-km-z]{32,44}$'),
 transaction_id text NOT NULL CHECK(transaction_id ~ '^[1-9A-HJ-NP-Za-km-z]{64,88}$'),
 evidence_id uuid NOT NULL UNIQUE,
 record jsonb NOT NULL CHECK(jsonb_typeof(record)='object' AND octet_length(record::text)<=16384),
 record_sha256 text NOT NULL CHECK(record_sha256 ~ '^[0-9a-f]{64}$'),
 verified_at timestamptz NOT NULL, expires_at timestamptz NOT NULL,
 CHECK(expires_at>verified_at AND expires_at<=verified_at+interval '15 minutes'),
 CHECK((kind='original')=(original_id IS NULL)),
 CHECK(record_sha256=encode(sha256(convert_to(record::text,'UTF8')),'hex'))
);
-- Hashes use PostgreSQL jsonb::text UTF-8, not a client JSON serializer.
CREATE TABLE agent_tokens.revocations (
 receipt_id uuid PRIMARY KEY REFERENCES agent_tokens.receipts(id),
 recorded_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE agent_tokens.bindings (
 id uuid PRIMARY KEY REFERENCES agent_tokens.receipts(id),
 agent_id uuid NOT NULL REFERENCES public.agents(id), creator_id uuid NOT NULL,
 network text NOT NULL, mint text NOT NULL, kind text NOT NULL,
 original_id uuid REFERENCES agent_tokens.bindings(id),
 evidence_id uuid NOT NULL UNIQUE,
 record jsonb NOT NULL, record_sha256 text NOT NULL,
 recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(network,mint),
 CHECK(kind IN ('original','successor')),
 CHECK((kind='original')=(original_id IS NULL))
);
CREATE UNIQUE INDEX one_original_per_agent ON agent_tokens.bindings(agent_id) WHERE kind='original';
CREATE UNIQUE INDEX one_successor_authorization ON agent_tokens.bindings
 ((record->'candidate'->>'authorization_id')) WHERE kind='successor';
CREATE UNIQUE INDEX one_successor_disclosure ON agent_tokens.bindings
 ((record->'candidate'->>'disclosure_id')) WHERE kind='successor';
CREATE FUNCTION agent_tokens.immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'token_history_immutable' USING ERRCODE='42501'; END;
$$;
CREATE FUNCTION agent_tokens.lock_revocation() RETURNS trigger LANGUAGE plpgsql
SET search_path=pg_catalog AS $$
BEGIN
 PERFORM 1 FROM agent_tokens.receipts WHERE id=NEW.receipt_id FOR UPDATE;
 RETURN NEW;
END;
$$;
CREATE TRIGGER revocation_serialization BEFORE INSERT ON agent_tokens.revocations
 FOR EACH ROW EXECUTE FUNCTION agent_tokens.lock_revocation();
CREATE TRIGGER receipts_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON agent_tokens.receipts
 FOR EACH STATEMENT EXECUTE FUNCTION agent_tokens.immutable();
CREATE TRIGGER revocations_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON agent_tokens.revocations
 FOR EACH STATEMENT EXECUTE FUNCTION agent_tokens.immutable();
CREATE TRIGGER bindings_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON agent_tokens.bindings
 FOR EACH STATEMENT EXECUTE FUNCTION agent_tokens.immutable();
-- actor is supplied only by the authenticated application principal, never body
-- JSON. Receipt is owner-bound; operator/viewer/administrative roles imply no right.
CREATE FUNCTION agent_tokens.confirm(actor uuid, receipt uuid, expected_record jsonb)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE r agent_tokens.receipts; root agent_tokens.bindings; q jsonb; p jsonb; a jsonb;
BEGIN
 SELECT * INTO STRICT r FROM agent_tokens.receipts WHERE id=receipt;
 PERFORM 1 FROM public.agents WHERE id=r.agent_id FOR UPDATE;
 PERFORM 1 FROM agent_tokens.receipts WHERE id=receipt FOR UPDATE;
 -- Recheck live ACL after serialization with ownership changes.
 IF actor<>r.creator_id OR NOT EXISTS(SELECT 1 FROM public.agent_access
   WHERE agent_id=r.agent_id AND principal_id=actor AND role='owner')
   OR EXISTS(SELECT 1 FROM public.application_epoch) THEN
  RAISE EXCEPTION 'token_creator_authorization_required' USING ERRCODE='42501';
 END IF;
 IF r.record IS DISTINCT FROM expected_record OR
   r.record_sha256<>encode(sha256(convert_to(expected_record::text,'UTF8')),'hex') OR
   r.verified_at>clock_timestamp() OR r.expires_at<=clock_timestamp() OR
   EXISTS(SELECT 1 FROM agent_tokens.revocations WHERE receipt_id=receipt) THEN
  RAISE EXCEPTION 'token_verification_receipt_invalid' USING ERRCODE='22023';
 END IF;
 SELECT body INTO p FROM ecosystem_social.purposes WHERE agent_id=r.agent_id
   AND creator_id=actor ORDER BY revision DESC LIMIT 1;
 SELECT body INTO q FROM ecosystem_social.owner_reviews WHERE agent_id=r.agent_id
   AND creator_id=actor AND kind='qualification' ORDER BY recorded_at DESC,id DESC LIMIT 1;
 IF (p IS NOT NULL AND q IS NOT NULL AND (p->>'id')::uuid=r.purpose_id AND
   (q->>'id')::uuid=r.qualification_id AND q->'purpose'=p AND
   q->>'decision'='qualified' AND (q->>'reviewer_id')::uuid=actor AND
   (q->>'reviewed_at')::timestamptz<=clock_timestamp() AND
   (q->>'expires_at')::timestamptz>clock_timestamp() AND
   EXISTS(SELECT 1 FROM ecosystem_social.qualification_policies policy
     WHERE policy.id=(q->>'policy_id')::uuid AND policy.version=(q->>'policy_version')::integer
       AND enabled AND actor=ANY(reviewers) AND
       (q->>'expires_at')::timestamptz-(q->>'reviewed_at')::timestamptz<=make_interval(secs=>policy.max_validity_seconds)
       AND NOT EXISTS(SELECT 1 FROM ecosystem_social.qualification_policies newer
         WHERE newer.id=policy.id AND newer.version>policy.version))) IS NOT TRUE THEN
  RAISE EXCEPTION 'token_current_qualification_required' USING ERRCODE='42501';
 END IF;
 a:=CASE WHEN r.kind='original' THEN r.record->'association' ELSE r.record->'candidate'->'successor' END;
 IF (a->>'id'=r.id::text AND a->>'agent_id'=r.agent_id::text AND
     a->>'creator_id'=r.creator_id::text AND a->>'purpose_revision_id'=r.purpose_id::text AND
     a->>'qualification_review_id'=r.qualification_id::text AND a->>'network'=r.network AND
     a->>'mint'=r.mint AND a->>'transaction'=r.transaction_id AND a->>'evidence_id'=r.evidence_id::text AND
     (r.record->>'confirmed_at')::timestamptz<=r.verified_at AND
     (r.record->>'confirmed_at')::timestamptz>=(q->>'reviewed_at')::timestamptz) IS NOT TRUE THEN
  RAISE EXCEPTION 'token_evidence_snapshot_mismatch' USING ERRCODE='22023';
 END IF;
 IF r.kind='successor' THEN
  SELECT * INTO STRICT root FROM agent_tokens.bindings WHERE id=r.original_id;
  IF root.kind<>'original' OR root.agent_id<>r.agent_id OR root.network<>r.network OR
     (r.record->'candidate'->'original' IS DISTINCT FROM root.record->'association') THEN
   RAISE EXCEPTION 'token_original_binding_mismatch' USING ERRCODE='22023';
  END IF;
  IF ((r.record->>'authorization_expires_at')::timestamptz>clock_timestamp() AND
      (r.record->>'disclosure_verified_at')::timestamptz<=r.verified_at AND
      (r.record->>'disclosed_at')::timestamptz>=(r.record->>'confirmed_at')::timestamptz AND
      (r.record->>'disclosed_at')::timestamptz<=(r.record->>'disclosure_verified_at')::timestamptz) IS NOT TRUE THEN
   RAISE EXCEPTION 'token_successor_receipt_stale' USING ERRCODE='22023';
  END IF;
 END IF;
 -- Global mint/evidence uniqueness and the partial index arbitrate cross-agent
 -- races. Never UPSERT: an attempted replacement cannot rewrite old history.
 INSERT INTO agent_tokens.bindings(id,agent_id,creator_id,network,mint,kind,original_id,evidence_id,record,record_sha256)
 VALUES(r.id,r.agent_id,r.creator_id,r.network,r.mint,r.kind,r.original_id,r.evidence_id,r.record,r.record_sha256);
 RETURN r.id;
END;
$$;
REVOKE ALL ON ALL TABLES IN SCHEMA agent_tokens FROM PUBLIC;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA agent_tokens FROM PUBLIC;
COMMIT;
