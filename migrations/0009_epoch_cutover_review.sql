-- Applied only to disposable review databases in this milestone; never activates a row.
ALTER TABLE application_epoch DROP CONSTRAINT application_epoch_state_check;
ALTER TABLE application_epoch ADD CONSTRAINT application_epoch_state_check CHECK(state IN ('prepared','active_read_only'));
ALTER TABLE application_epoch ADD COLUMN revision bigint NOT NULL DEFAULT 0 CHECK(revision>=0);
ALTER TABLE application_epoch ADD COLUMN approval_id uuid;
CREATE TABLE epoch_transitions (
    approval_id uuid PRIMARY KEY CHECK(approval_id <> '00000000-0000-0000-0000-000000000000'),
    epoch_id uuid NOT NULL REFERENCES application_epoch(id),
    revision bigint NOT NULL CHECK(revision>0),
    operation text NOT NULL CHECK(operation IN ('activate_read_only','rollback')),
    approved_by uuid NOT NULL CHECK(approved_by <> '00000000-0000-0000-0000-000000000000'),
    plan_sha256 text NOT NULL CHECK(plan_sha256 ~ '^[0-9a-f]{64}$'),
    from_state text NOT NULL CHECK(from_state IN ('prepared','active_read_only')),
    to_state text NOT NULL CHECK(to_state IN ('prepared','active_read_only')),
    occurred_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(epoch_id,revision),
    CHECK((operation='activate_read_only' AND from_state='prepared' AND to_state='active_read_only')
       OR (operation='rollback' AND from_state='active_read_only' AND to_state='prepared'))
);
CREATE FUNCTION preserve_epoch_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'epoch_transition_audit_is_append_only';
END;
$$;
CREATE TRIGGER epoch_transition_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON epoch_transitions
    FOR EACH STATEMENT EXECUTE FUNCTION preserve_epoch_transition();
