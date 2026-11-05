-- Proposed feature-only service role. Apply solely in reviewed disposable tests.
-- Creation of this NOINHERIT/NOSUPERUSER role belongs to the test administrator.
-- No role memberships, public table writes, sequence rights or financial grants.
BEGIN;
GRANT USAGE ON SCHEMA public,ecosystem_social TO supa_ecosystem_runtime;
GRANT SELECT ON public.agents,public.agent_access,public.application_epoch,
 public.activity,public.economic_jobs,public.economic_audit,public._sqlx_migrations TO supa_ecosystem_runtime;
GRANT SELECT,INSERT ON ecosystem_social.purposes,ecosystem_social.consents,ecosystem_social.sources,
 ecosystem_social.preparations,ecosystem_social.prepared_events,ecosystem_social.owner_reviews TO supa_ecosystem_runtime;
GRANT SELECT ON ecosystem_social.qualification_policies TO supa_ecosystem_runtime;
GRANT EXECUTE ON FUNCTION ecosystem_social.lock_agent(uuid) TO supa_ecosystem_runtime;
GRANT EXECUTE ON FUNCTION public.work_review_canonical(jsonb) TO supa_ecosystem_runtime;
COMMIT;
