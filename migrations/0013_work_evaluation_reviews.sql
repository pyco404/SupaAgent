-- Evaluation-only review. No session, job, balance, grant or authorization changes.
-- Explicitly unavailable to application epochs until a separate scope is designed.
CREATE FUNCTION work_review_canonical(value jsonb) RETURNS text
LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE result text;
BEGIN
 CASE jsonb_typeof(value)
 WHEN 'object' THEN
  SELECT '{'||coalesce(string_agg(to_jsonb(key)::text||':'||work_review_canonical(val),',' ORDER BY key COLLATE "C"),'')||'}'
   INTO result FROM jsonb_each(value) AS item(key,val);
 WHEN 'array' THEN
  SELECT '['||coalesce(string_agg(work_review_canonical(val),',' ORDER BY ord),'')||']'
   INTO result FROM jsonb_array_elements(value) WITH ORDINALITY AS item(val,ord);
 ELSE result:=value::text;
 END CASE;
 RETURN result;
END;
$$;

CREATE TABLE work_evaluation_reviews (
 request_id uuid PRIMARY KEY,
 session_id uuid NOT NULL UNIQUE REFERENCES live_sessions(id),
 agent_id uuid NOT NULL REFERENCES agents(id),
 reviewer_principal_id uuid NOT NULL,
 decision text NOT NULL CHECK(decision IN ('accept_evaluation_only','reject_evaluation_only')),
 report_sha256 text NOT NULL CHECK(report_sha256 ~ '^[0-9a-f]{64}$'),
 confirmation jsonb NOT NULL,
 reviewed_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
REVOKE ALL ON work_evaluation_reviews FROM PUBLIC;

CREATE FUNCTION work_review_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE s live_sessions; c jsonb:=NEW.confirmation; now_at timestamptz:=clock_timestamp();
BEGIN
 IF EXISTS(SELECT 1 FROM application_epoch) THEN
  RAISE EXCEPTION 'work_review_epoch_not_authorized' USING ERRCODE='42501';
 END IF;
 SELECT * INTO s FROM live_sessions WHERE id=NEW.session_id;
 IF (NOT FOUND OR s.agent_id<>NEW.agent_id OR s.approved_by<>NEW.reviewer_principal_id
  OR s.status<>'awaiting_approval' OR s.data->>'status'<>'awaiting_approval'
  OR s.data->>'id'<>NEW.session_id::text OR s.data->>'agent_id'<>NEW.agent_id::text
  OR s.data->>'approved_by'<>NEW.reviewer_principal_id::text
  OR s.data->'work'->'report' IS NULL OR s.data->'work'->'report'='null'::jsonb
  OR (s.data->>'generation_requests')::bigint<>1 OR s.data->'usage' IS NULL OR s.data->'usage'='null'::jsonb
  OR s.data->'finished_at' IS NULL OR s.data->'finished_at'='null'::jsonb
  OR NOT EXISTS(SELECT 1 FROM agent_access WHERE agent_id=NEW.agent_id AND principal_id=NEW.reviewer_principal_id AND role='owner')) IS NOT FALSE THEN
  RAISE EXCEPTION 'work_review_not_permitted' USING ERRCODE='42501';
 END IF;
 IF (NEW.report_sha256<>encode(sha256(convert_to(work_review_canonical(s.data->'work'->'report'),'UTF8')),'hex')
  OR NEW.request_id=NEW.session_id OR NEW.request_id='00000000-0000-0000-0000-000000000000'::uuid
  OR jsonb_typeof(c)<>'object' OR (SELECT count(*) FROM jsonb_object_keys(c))<>6
  OR NOT (c ?& ARRAY['request_id','decision','report_sha256','confirm_review','consented_at','expires_at'])
  OR c->>'request_id'<>NEW.request_id::text OR c->>'decision'<>NEW.decision
  OR c->>'report_sha256'<>NEW.report_sha256 OR c->'confirm_review'<>'true'::jsonb
  OR NOT ((c->>'consented_at')::timestamptz<=now_at AND (c->>'expires_at')::timestamptz>now_at
       AND (c->>'expires_at')::timestamptz>(c->>'consented_at')::timestamptz
       AND (c->>'expires_at')::timestamptz<=(c->>'consented_at')::timestamptz+interval '5 minutes')) IS NOT FALSE THEN
  RAISE EXCEPTION 'work_review_stale_or_mismatched' USING ERRCODE='23514';
 END IF;
 NEW.reviewed_at:=now_at;
 RETURN NEW;
END;
$$;
CREATE TRIGGER work_review_bound BEFORE INSERT ON work_evaluation_reviews FOR EACH ROW EXECUTE FUNCTION work_review_guard();
CREATE FUNCTION work_review_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'work_review_is_immutable' USING ERRCODE='42501'; END;
$$;
CREATE TRIGGER work_review_append_only BEFORE UPDATE OR DELETE OR TRUNCATE ON work_evaluation_reviews
FOR EACH STATEMENT EXECUTE FUNCTION work_review_immutable();
