-- PowerFixer Schema Redesign
-- This migration drops old tables and creates the new unified schema.

-- Drop old tables in dependency order
DROP TABLE IF EXISTS dedupe_closures CASCADE;
DROP TABLE IF EXISTS dedupe_results CASCADE;
DROP TABLE IF EXISTS triage_history CASCADE;
DROP TABLE IF EXISTS triage_agents CASCADE;
DROP TABLE IF EXISTS local_agents CASCADE;
DROP TABLE IF EXISTS agent_assignments CASCADE;
DROP TABLE IF EXISTS action_logs CASCADE;
DROP TABLE IF EXISTS inbox_state CASCADE;
DROP TABLE IF EXISTS issues CASCADE;

-- Drop old enum types (keep issue_provider - used by provider_configs)
DROP TYPE IF EXISTS issue_state CASCADE;
DROP TYPE IF EXISTS agent_task_type CASCADE;
DROP TYPE IF EXISTS local_agent_status CASCADE;
DROP TYPE IF EXISTS agent_task_state CASCADE;

-- Create new enum types
CREATE TYPE agent_type AS ENUM ('fix', 'dedupe', 'triage');
CREATE TYPE execution_mode AS ENUM ('local', 'remote');
CREATE TYPE agent_task_state AS ENUM ('queued', 'in_progress', 'succeeded', 'failed');
CREATE TYPE triage_result_type AS ENUM ('candidate', 'rejected');
CREATE TYPE issue_action_type AS ENUM ('comment', 'close', 'label_add', 'label_remove', 'assign', 'other');

-- Issues table
CREATE TABLE issues (
    id SERIAL PRIMARY KEY,
    provider_config_id INTEGER NOT NULL REFERENCES provider_configs(id),
    external_id VARCHAR(255) NOT NULL,
    external_url TEXT,
    project VARCHAR(255) NOT NULL,
    title TEXT,
    labels TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider_config_id, project, external_id)
);

-- Unified agents table
CREATE TABLE agents (
    id SERIAL PRIMARY KEY,
    agent_type agent_type NOT NULL,
    execution_mode execution_mode NOT NULL,
    trigger_issue_id INTEGER REFERENCES issues(id) ON DELETE SET NULL,
    task_id VARCHAR(255),
    callback_token VARCHAR(255) NOT NULL,
    prompt TEXT NOT NULL,
    task_state agent_task_state NOT NULL DEFAULT 'queued',
    session_url TEXT,
    branch_name VARCHAR(255),
    pr_url TEXT,
    summary TEXT,
    pid INTEGER,
    log_path TEXT,
    started_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Agent status updates
CREATE TABLE agent_status_updates (
    id SERIAL PRIMARY KEY,
    agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    state VARCHAR(50) NOT NULL,
    message TEXT,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Issue actions log
CREATE TABLE issue_actions (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    action_type issue_action_type NOT NULL,
    details JSONB,
    performed_by VARCHAR(100) NOT NULL,
    performed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Triage runs
CREATE TABLE triage_runs (
    id SERIAL PRIMARY KEY,
    started_at TIMESTAMPTZ NOT NULL,
    min_issue_number INTEGER NOT NULL,
    max_issue_number INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Triage run agents junction
CREATE TABLE triage_run_agents (
    id SERIAL PRIMARY KEY,
    triage_run_id INTEGER NOT NULL REFERENCES triage_runs(id) ON DELETE CASCADE,
    agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    issue_numbers INTEGER[] NOT NULL,
    UNIQUE(triage_run_id, agent_id)
);

-- Triage results
CREATE TABLE triage_results (
    id SERIAL PRIMARY KEY,
    triage_run_id INTEGER NOT NULL REFERENCES triage_runs(id) ON DELETE CASCADE,
    agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    issue_number INTEGER NOT NULL,
    result triage_result_type NOT NULL,
    reason TEXT NOT NULL,
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(issue_number, triage_run_id)
);

-- Dedupe runs
CREATE TABLE dedupe_runs (
    id SERIAL PRIMARY KEY,
    agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    canonical_issue_url TEXT NOT NULL,
    analysis_summary TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dedupe duplicates
CREATE TABLE dedupe_duplicates (
    id SERIAL PRIMARY KEY,
    dedupe_run_id INTEGER NOT NULL REFERENCES dedupe_runs(id) ON DELETE CASCADE,
    issue_url TEXT NOT NULL,
    confidence REAL NOT NULL,
    reason TEXT NOT NULL
);

-- Dedupe closures
CREATE TABLE dedupe_closures (
    id SERIAL PRIMARY KEY,
    dedupe_run_id INTEGER NOT NULL REFERENCES dedupe_runs(id) ON DELETE CASCADE,
    closed_issue_url TEXT NOT NULL,
    closed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_by VARCHAR(100) NOT NULL
);

-- Agent inbox state
CREATE TABLE agent_inbox_state (
    id SERIAL PRIMARY KEY,
    agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(agent_id)
);

-- Issue inbox state
CREATE TABLE issue_inbox_state (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(issue_id)
);

-- Indexes
CREATE INDEX idx_issues_provider_config ON issues(provider_config_id);
CREATE INDEX idx_issues_labels ON issues USING GIN(labels);
CREATE INDEX idx_agents_type ON agents(agent_type);
CREATE INDEX idx_agents_state ON agents(task_state);
CREATE INDEX idx_agents_trigger_issue ON agents(trigger_issue_id);
CREATE INDEX idx_agents_callback_token ON agents(callback_token);
CREATE INDEX idx_agent_status_updates_agent ON agent_status_updates(agent_id);
CREATE INDEX idx_issue_actions_issue ON issue_actions(issue_id);
CREATE INDEX idx_issue_actions_type ON issue_actions(action_type);
CREATE INDEX idx_triage_run_agents_run ON triage_run_agents(triage_run_id);
CREATE INDEX idx_triage_run_agents_agent ON triage_run_agents(agent_id);
CREATE INDEX idx_triage_results_run ON triage_results(triage_run_id);
CREATE INDEX idx_triage_results_issue ON triage_results(issue_number);
CREATE INDEX idx_dedupe_runs_agent ON dedupe_runs(agent_id);
CREATE INDEX idx_dedupe_duplicates_run ON dedupe_duplicates(dedupe_run_id);
CREATE INDEX idx_dedupe_closures_run ON dedupe_closures(dedupe_run_id);

-- Re-insert default provider config if needed
INSERT INTO provider_configs (provider, organization, base_url, is_default)
VALUES ('github', 'example-org', 'https://github.com', TRUE)
ON CONFLICT (provider, organization) DO NOTHING;
