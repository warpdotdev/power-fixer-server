-- Revert string external_ids back to numeric issue IDs
-- Note: This may fail if non-numeric external_ids exist

-- triage_results: external_id -> issue_number
ALTER TABLE triage_results ADD COLUMN issue_number INTEGER;

UPDATE triage_results SET issue_number = external_id::INTEGER;

ALTER TABLE triage_results DROP CONSTRAINT IF EXISTS triage_results_external_id_triage_run_id_key;

ALTER TABLE triage_results 
    ALTER COLUMN issue_number SET NOT NULL,
    DROP COLUMN external_id;

ALTER TABLE triage_results ADD CONSTRAINT triage_results_issue_number_triage_run_id_key 
    UNIQUE(issue_number, triage_run_id);

DROP INDEX IF EXISTS idx_triage_results_external_id;
CREATE INDEX idx_triage_results_issue ON triage_results(issue_number);

-- triage_run_agents: external_ids -> issue_numbers
ALTER TABLE triage_run_agents ADD COLUMN issue_numbers INTEGER[];

UPDATE triage_run_agents 
SET issue_numbers = (
    SELECT array_agg(n::INTEGER) 
    FROM unnest(external_ids) AS n
);

ALTER TABLE triage_run_agents 
    ALTER COLUMN issue_numbers SET NOT NULL,
    DROP COLUMN external_ids;

-- triage_runs: min/max external_id -> issue_number
ALTER TABLE triage_runs 
    ADD COLUMN min_issue_number INTEGER,
    ADD COLUMN max_issue_number INTEGER;

UPDATE triage_runs 
SET min_issue_number = min_external_id::INTEGER,
    max_issue_number = max_external_id::INTEGER;

ALTER TABLE triage_runs 
    ALTER COLUMN min_issue_number SET NOT NULL,
    ALTER COLUMN max_issue_number SET NOT NULL,
    DROP COLUMN min_external_id,
    DROP COLUMN max_external_id;
