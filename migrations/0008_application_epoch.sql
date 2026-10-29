-- Empty on existing databases: never infer ownership, import history, or activate an epoch.
CREATE TABLE application_epoch (
    singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
    id uuid NOT NULL UNIQUE CHECK(id <> 'ad91e475-f1da-495a-8247-139643d2d8b1'),
    accounting_namespace uuid NOT NULL UNIQUE CHECK(accounting_namespace <> 'ad91e475-f1da-495a-8247-139643d2d8b1'),
    label text NOT NULL CHECK(length(label) BETWEEN 1 AND 80),
    kind text NOT NULL CHECK(kind = 'development'),
    state text NOT NULL CHECK(state = 'prepared'),
    historical_database text NOT NULL CHECK(historical_database = 'unrecovered'),
    created_at timestamptz NOT NULL DEFAULT now()
);
