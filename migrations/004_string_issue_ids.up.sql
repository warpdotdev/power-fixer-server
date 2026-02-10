-- Convert numeric issue IDs to string-based external_ids
-- This supports non-numeric identifiers like Linear's "WAR-1234"

-- triage_runs: min/max issue number -> external_id
ALTER TABLE triage_runs 
    ADD COLUMN min_external_id TEXT,
    ADD COLUMN max_external_id TEXT;

UPDATE triage_runs 
SET min_external_id = min_issue_number::TEXT,
    max_external_id = max_issue_number::TEXT;

ALTER TABLE triage_runs 
    ALTER COLUMN min_external_id SET NOT NULL,
    ALTER COLUMN max_external_id SET NOT NULL,
    DROP COLUMN min_issue_number,
    DROP COLUMN max_issue_number;

-- triage_run_agents: issue_numbers -> external_ids
ALTER TABLE triage_run_agents ADD COLUMN external_ids TEXT[];

UPDATE triage_run_agents 
SET external_ids = (
    SELECT array_agg(n::TEXT) 
    FROM unnest(issue_numbers) AS n
);

ALTER TABLE triage_run_agents 
    ALTER COLUMN external_ids SET NOT NULL,
    DROP COLUMN issue_numbers;

-- triage_results: issue_number -> external_id
ALTER TABLE triage_results ADD COLUMN external_id TEXT;

UPDATE triage_results SET external_id = issue_number::TEXT;

-- Drop old unique constraint and create new one
ALTER TABLE triage_results DROP CONSTRAINT IF EXISTS triage_results_issue_number_triage_run_id_key;

ALTER TABLE triage_results 
    ALTER COLUMN external_id SET NOT NULL,
    DROP COLUMN issue_number;

ALTER TABLE triage_results ADD CONSTRAINT triage_results_external_id_triage_run_id_key 
    UNIQUE(external_id, triage_run_id);

-- Update index
DROP INDEX IF EXISTS idx_triage_results_issue;
CREATE INDEX idx_triage_results_external_id ON triage_results(external_id);
