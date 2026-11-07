-- Explicit synthetic-pilot extension. No automatic migration or live grants.
BEGIN;
CREATE SCHEMA agent_avatars;
REVOKE ALL ON SCHEMA agent_avatars FROM PUBLIC;
CREATE TABLE agent_avatars.assets (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), owner_id uuid NOT NULL,
 request_id uuid NOT NULL, kind text NOT NULL CHECK(kind IN ('upload','default')),
 png bytea NOT NULL CHECK(octet_length(png) BETWEEN 67 AND 1100000),
 sha256 text NOT NULL CHECK(sha256 ~ '^[0-9a-f]{64}$'),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 expires_at timestamptz NOT NULL DEFAULT clock_timestamp()+interval '1 hour',
 UNIQUE(owner_id,request_id), CHECK(sha256=encode(sha256(png),'hex')),
 CHECK(substr(png,1,8)=decode('89504e470d0a1a0a','hex')),
 CHECK(substr(png,17,8)=decode('0000020000000200','hex'))
);
CREATE TABLE agent_avatars.revisions (
 agent_id uuid NOT NULL REFERENCES public.agents(id), revision integer NOT NULL CHECK(revision>0),
 asset_id uuid NOT NULL REFERENCES agent_avatars.assets(id), actor uuid NOT NULL,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(), PRIMARY KEY(agent_id,revision)
);
CREATE TABLE agent_avatars.public_approvals (
 agent_id uuid PRIMARY KEY REFERENCES public.agents(id), revision integer NOT NULL,
 approved boolean NOT NULL, actor uuid NOT NULL,
 FOREIGN KEY(agent_id,revision) REFERENCES agent_avatars.revisions(agent_id,revision)
);
CREATE TABLE agent_avatars.token_images (
 agent_id uuid PRIMARY KEY REFERENCES public.agents(id), revision integer NOT NULL,
 asset_id uuid NOT NULL REFERENCES agent_avatars.assets(id), sha256 text NOT NULL,
 actor uuid NOT NULL, confirmed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 FOREIGN KEY(agent_id,revision) REFERENCES agent_avatars.revisions(agent_id,revision)
);
CREATE TABLE agent_avatars.commands (
 actor uuid NOT NULL, request_id uuid NOT NULL, body jsonb NOT NULL, result jsonb NOT NULL,
 PRIMARY KEY(actor,request_id)
);
CREATE TABLE agent_avatars.upload_limits (
 actor uuid PRIMARY KEY, hour timestamptz NOT NULL, uploads integer NOT NULL CHECK(uploads BETWEEN 1 AND 20)
);
REVOKE ALL ON ALL TABLES IN SCHEMA agent_avatars FROM PUBLIC;
CREATE TRIGGER revisions_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON agent_avatars.revisions
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
CREATE TRIGGER token_images_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON agent_avatars.token_images
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
CREATE TRIGGER commands_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON agent_avatars.commands
 FOR EACH STATEMENT EXECUTE FUNCTION ecosystem_social.immutable();
CREATE FUNCTION agent_avatars.asset_bytes_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'avatar_bytes_immutable' USING ERRCODE='42501'; END;
$$;
CREATE TRIGGER asset_bytes_immutable BEFORE UPDATE ON agent_avatars.assets
 FOR EACH STATEMENT EXECUTE FUNCTION agent_avatars.asset_bytes_immutable();

CREATE FUNCTION agent_avatars.public_profile(agent uuid) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog AS $$
 SELECT jsonb_build_object('agent_id',f.agent_id,'name',f.configuration->>'name',
  'summary',p.body->>'public_summary','revision',r.revision)
 FROM agent_factory.submissions f
 JOIN public.agent_access x ON x.agent_id=f.agent_id AND x.principal_id=f.creator_id AND x.role='owner'
 JOIN LATERAL (SELECT * FROM ecosystem_social.consents c WHERE c.agent_id=f.agent_id AND c.creator_id=f.creator_id ORDER BY revision DESC LIMIT 1) c ON true
 JOIN LATERAL (SELECT * FROM ecosystem_social.purposes p WHERE p.agent_id=f.agent_id AND p.creator_id=f.creator_id ORDER BY revision DESC LIMIT 1) p ON true
 JOIN LATERAL (SELECT * FROM agent_avatars.revisions r WHERE r.agent_id=f.agent_id ORDER BY revision DESC LIMIT 1) r ON true
 JOIN agent_avatars.public_approvals approval ON approval.agent_id=r.agent_id AND approval.revision=r.revision AND approval.approved
 WHERE f.agent_id=agent AND c.body->'public'='true'::jsonb AND c.body->'revoked'='false'::jsonb
 AND (c.body->>'valid_from')::timestamptz<=clock_timestamp()
 AND (c.body->>'expires_at')::timestamptz>clock_timestamp()
 AND c.body->'categories' ? 'agent_created'
 AND length(p.body->>'public_summary') BETWEEN 1 AND 280
$$;
CREATE FUNCTION agent_avatars.command(actor uuid, op text, b jsonb) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
#variable_conflict use_variable
DECLARE a agent_avatars.assets%ROWTYPE; agent uuid; request uuid; current_rev integer;
 rev integer; saved agent_avatars.commands%ROWTYPE; result jsonb; amount integer; digest text;
BEGIN
 IF op='public_directory' THEN
  RETURN jsonb_build_object('agents',coalesce((SELECT jsonb_agg(p) FROM
   (SELECT agent_avatars.public_profile(f.agent_id) p FROM agent_factory.submissions f ORDER BY recorded_at DESC LIMIT 100) q WHERE p IS NOT NULL),'[]'::jsonb));
 ELSIF op IN ('public_profile','public_asset') THEN
  agent=(b->>'agent_id')::uuid;result=agent_avatars.public_profile(agent);
  IF result IS NULL THEN RAISE EXCEPTION 'public_avatar_unavailable' USING ERRCODE='P0002'; END IF;
  IF op='public_profile' THEN RETURN result; END IF;
  IF (b->>'revision')::integer<>(result->>'revision')::integer THEN RAISE EXCEPTION 'avatar_version_unavailable' USING ERRCODE='P0002'; END IF;
  SELECT asset.* INTO a FROM agent_avatars.assets asset JOIN agent_avatars.revisions r ON r.asset_id=asset.id
   WHERE r.agent_id=agent AND r.revision=(result->>'revision')::integer;
  RETURN jsonb_build_object('png_hex',encode(a.png,'hex'),'sha256',a.sha256);
 END IF;
 IF actor IS NULL OR actor='00000000-0000-0000-0000-000000000000'::uuid
 OR EXISTS(SELECT 1 FROM public.application_epoch) THEN RAISE EXCEPTION 'avatar_scope_denied' USING ERRCODE='42501'; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('avatar_owner:'||actor::text,0));
 IF op IN ('upload','cleanup','remove_upload') THEN
  DELETE FROM agent_avatars.assets asset WHERE asset.owner_id=actor AND asset.expires_at<clock_timestamp()
   AND NOT EXISTS(SELECT 1 FROM agent_avatars.revisions WHERE asset_id=asset.id)
   AND NOT EXISTS(SELECT 1 FROM agent_avatars.token_images WHERE asset_id=asset.id);
  GET DIAGNOSTICS amount=ROW_COUNT;
  IF op='cleanup' THEN RETURN jsonb_build_object('removed',amount); END IF;
  IF op='remove_upload' THEN
   DELETE FROM agent_avatars.assets asset WHERE asset.id=(b->>'asset_id')::uuid AND asset.owner_id=actor
    AND NOT EXISTS(SELECT 1 FROM agent_avatars.revisions WHERE asset_id=asset.id)
    AND NOT EXISTS(SELECT 1 FROM agent_avatars.token_images WHERE asset_id=asset.id);
   RETURN jsonb_build_object('removed',FOUND);
  END IF;
  request=(b->>'request_id')::uuid;
  SELECT * INTO a FROM agent_avatars.assets WHERE owner_id=actor AND request_id=request;
  IF FOUND THEN
   IF a.sha256<>b->>'sha256' OR a.kind<>b->>'kind' THEN RAISE EXCEPTION 'avatar_upload_conflict' USING ERRCODE='40001'; END IF;
   RETURN jsonb_build_object('asset_id',a.id,'sha256',a.sha256,'kind',a.kind);
  END IF;
  IF (SELECT count(*) FROM agent_avatars.assets WHERE owner_id=actor)>=100
   OR (SELECT coalesce(sum(octet_length(png)),0) FROM agent_avatars.assets WHERE owner_id=actor)+length(b->>'png_hex')/2>20971520 THEN
   RAISE EXCEPTION 'avatar_storage_quota' USING ERRCODE='54000';
  END IF;
  INSERT INTO agent_avatars.upload_limits VALUES(actor,date_trunc('hour',clock_timestamp()),1)
   ON CONFLICT ON CONSTRAINT upload_limits_pkey DO UPDATE SET hour=excluded.hour, uploads=CASE WHEN agent_avatars.upload_limits.hour=excluded.hour THEN agent_avatars.upload_limits.uploads+1 ELSE 1 END;
  INSERT INTO agent_avatars.assets(owner_id,request_id,kind,png,sha256)
   VALUES(actor,request,b->>'kind',decode(b->>'png_hex','hex'),b->>'sha256') RETURNING * INTO a;
  RETURN jsonb_build_object('asset_id',a.id,'sha256',a.sha256,'kind',a.kind);
 ELSIF op='read_asset' THEN
  SELECT * INTO a FROM agent_avatars.assets WHERE id=(b->>'asset_id')::uuid AND owner_id=actor;
  IF NOT FOUND OR EXISTS(SELECT 1 FROM agent_avatars.revisions r WHERE r.asset_id=a.id AND NOT EXISTS(SELECT 1 FROM public.agent_access WHERE agent_id=r.agent_id AND principal_id=actor AND role='owner'))
   OR (a.expires_at<clock_timestamp() AND NOT EXISTS(SELECT 1 FROM agent_avatars.revisions WHERE asset_id=a.id)) THEN
   RAISE EXCEPTION 'private_avatar_denied' USING ERRCODE='42501';
  END IF;
  RETURN jsonb_build_object('png_hex',encode(a.png,'hex'),'sha256',a.sha256);
 END IF;
 agent=(b->>'agent_id')::uuid;
 IF NOT EXISTS(SELECT 1 FROM public.agent_access WHERE agent_id=agent AND principal_id=actor AND role='owner')
 OR NOT EXISTS(SELECT 1 FROM agent_factory.submissions WHERE agent_id=agent AND creator_id=actor) THEN
  RAISE EXCEPTION 'avatar_owner_required' USING ERRCODE='42501';
 END IF;
 SELECT coalesce(max(revision),0) INTO current_rev FROM agent_avatars.revisions WHERE agent_id=agent;
 IF op='metadata' THEN
  SELECT asset.* INTO a FROM agent_avatars.assets asset JOIN agent_avatars.revisions r ON r.asset_id=asset.id WHERE r.agent_id=agent AND r.revision=current_rev;
  RETURN jsonb_build_object('default_seed',(SELECT configuration->>'request_id' FROM agent_factory.submissions WHERE agent_id=agent),'revision',current_rev,'asset_id',a.id,'sha256',a.sha256,'kind',coalesce(a.kind,'default'),
   'token_image',(SELECT jsonb_build_object('revision',revision,'sha256',sha256) FROM agent_avatars.token_images WHERE agent_id=agent));
 END IF;
 request=(b->>'request_id')::uuid;
 IF request IS NULL OR request='00000000-0000-0000-0000-000000000000'::uuid THEN RAISE EXCEPTION 'avatar_request_required' USING ERRCODE='42501'; END IF;
 SELECT * INTO saved FROM agent_avatars.commands c WHERE c.actor=command.actor AND c.request_id=request;
 IF FOUND THEN
  IF saved.body<>jsonb_build_object('op',op,'body',b) THEN RAISE EXCEPTION 'avatar_command_conflict' USING ERRCODE='40001'; END IF;
  RETURN saved.result;
 END IF;
 IF (b->>'expected_revision')::integer IS DISTINCT FROM current_rev THEN RAISE EXCEPTION 'stale_avatar_revision' USING ERRCODE='40001'; END IF;
 IF op='set_avatar' THEN
  IF current_rev>=50 THEN RAISE EXCEPTION 'avatar_revision_quota' USING ERRCODE='54000'; END IF;
  SELECT * INTO a FROM agent_avatars.assets WHERE id=(b->>'asset_id')::uuid AND owner_id=actor;
  IF NOT FOUND OR (a.expires_at<clock_timestamp() AND NOT EXISTS(SELECT 1 FROM agent_avatars.revisions WHERE asset_id=a.id))
   OR EXISTS(SELECT 1 FROM agent_avatars.revisions WHERE asset_id=a.id AND agent_id<>agent) THEN RAISE EXCEPTION 'avatar_upload_denied' USING ERRCODE='42501'; END IF;
  INSERT INTO agent_avatars.revisions VALUES(agent,current_rev+1,a.id,actor,clock_timestamp());
  result=jsonb_build_object('revision',current_rev+1,'sha256',a.sha256,'asset_id',a.id,'kind',a.kind);
 ELSIF op='approve_public' THEN
  IF current_rev=0 OR jsonb_typeof(b->'approved')<>'boolean' THEN RAISE EXCEPTION 'avatar_approval_required' USING ERRCODE='42501'; END IF;
  INSERT INTO agent_avatars.public_approvals VALUES(agent,current_rev,(b->>'approved')::boolean,actor)
   ON CONFLICT(agent_id) DO UPDATE SET revision=excluded.revision,approved=excluded.approved,actor=excluded.actor;
  result=jsonb_build_object('avatar_approved',(b->>'approved')::boolean,'agent_listing_unchanged',true);
 ELSIF op='confirm_token_image' THEN
  SELECT asset.* INTO a FROM agent_avatars.assets asset JOIN agent_avatars.revisions r ON r.asset_id=asset.id WHERE r.agent_id=agent AND r.revision=current_rev;
  IF NOT FOUND OR b->'confirmed' IS DISTINCT FROM 'true'::jsonb OR a.sha256 IS DISTINCT FROM b->>'sha256'
   OR EXISTS(SELECT 1 FROM agent_avatars.token_images WHERE agent_id=agent) THEN RAISE EXCEPTION 'token_image_confirmation_conflict' USING ERRCODE='40001'; END IF;
  INSERT INTO agent_avatars.token_images VALUES(agent,current_rev,a.id,a.sha256,actor,clock_timestamp());
  result=jsonb_build_object('revision',current_rev,'sha256',a.sha256,'launch_enabled',false);
 ELSE RAISE EXCEPTION 'avatar_operation_denied' USING ERRCODE='42501';
 END IF;
 INSERT INTO agent_avatars.commands VALUES(actor,request,jsonb_build_object('op',op,'body',b),result);
 RETURN result;
END;
$$;
CREATE FUNCTION agent_avatars.create_agent(actor uuid,request uuid,config jsonb,candidate jsonb,purpose jsonb,consent jsonb,asset uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE agent uuid; original uuid;
BEGIN
 IF config->>'avatar_upload_id' IS DISTINCT FROM asset::text THEN RAISE EXCEPTION 'avatar_binding_mismatch' USING ERRCODE='42501'; END IF;
 agent=agent_factory.create_private(actor,request,config,candidate,purpose,consent);
 SELECT asset_id INTO original FROM agent_avatars.revisions WHERE agent_id=agent AND revision=1;
 IF FOUND THEN
  IF original<>asset THEN RAISE EXCEPTION 'avatar_creation_conflict' USING ERRCODE='40001'; END IF;
 ELSE
  PERFORM agent_avatars.command(actor,'set_avatar',jsonb_build_object('agent_id',agent,'asset_id',asset,'expected_revision',0,'request_id',request));
 END IF;
 RETURN agent;
END;
$$;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA agent_avatars FROM PUBLIC;
-- Grant only USAGE(schema) and EXECUTE(command, create_agent) to the reviewed
-- application-service role. Actor authority comes from server auth, not JSON.
COMMIT;
