-- Add addressed status tracking to dedupe_runs
ALTER TABLE dedupe_runs ADD COLUMN is_addressed BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE dedupe_runs ADD COLUMN addressed_at TIMESTAMPTZ;
