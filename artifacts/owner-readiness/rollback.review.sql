-- REVIEW ONLY. No approval is implied by generating this file.
-- Requires separately approved migration 9 and exact owner binding.
-- Supply psql variables approval_id, expected_revision, plan_sha256 after consent.
\set ON_ERROR_STOP on
BEGIN READ WRITE;
SET LOCAL statement_timeout = '10s';
SET LOCAL lock_timeout = '5s';
SELECT set_config('supa.review.approval', :'approval_id', true);
SELECT set_config('supa.review.revision', :'expected_revision', true);
SELECT set_config('supa.review.digest', :'plan_sha256', true);
DO $review$
DECLARE
    current_epoch application_epoch%ROWTYPE;
    consent uuid := current_setting('supa.review.approval')::uuid;
    expected bigint := current_setting('supa.review.revision')::bigint;
    digest text := current_setting('supa.review.digest');
BEGIN
    IF current_database() <> 'supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748' OR current_setting('default_transaction_read_only') <> 'on'
       OR expected < 0 OR consent = '00000000-0000-0000-0000-000000000000'
       OR digest !~ '^[0-9a-f]{64}$'
       OR (SELECT count(*) FROM _sqlx_migrations WHERE success) <> 9 THEN
        RAISE EXCEPTION 'cutover_target_or_approval_mismatch';
    END IF;
    SELECT * INTO STRICT current_epoch FROM application_epoch WHERE id='fd2f52e0-33fc-4c0d-893a-7b08b26ab748' FOR UPDATE;
    IF current_epoch.state <> 'active_read_only' OR current_epoch.revision <> expected
       OR current_epoch.kind <> 'development' OR current_epoch.historical_database <> 'unrecovered'
       OR current_epoch.accounting_namespace <> '641457b2-5fd8-4b31-b85f-c58839626831'
       OR EXISTS(SELECT 1 FROM epoch_transitions WHERE approval_id=consent) THEN
        RAISE EXCEPTION 'cutover_state_or_replay_mismatch';
    END IF;
    INSERT INTO epoch_transitions(approval_id,epoch_id,revision,operation,approved_by,plan_sha256,from_state,to_state)
    VALUES(consent,current_epoch.id,expected+1,'rollback','6fd863e1-745d-4a06-b68e-eea69e854a69',digest,'active_read_only','prepared');
    UPDATE application_epoch SET state='prepared', revision=expected+1, approval_id=consent WHERE id=current_epoch.id;
END;
$review$;
COMMIT;

