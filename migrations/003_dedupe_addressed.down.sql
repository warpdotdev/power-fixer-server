-- Remove addressed status tracking from dedupe_runs
ALTER TABLE dedupe_runs DROP COLUMN IF EXISTS addressed_at;
ALTER TABLE dedupe_runs DROP COLUMN IF EXISTS is_addressed;
