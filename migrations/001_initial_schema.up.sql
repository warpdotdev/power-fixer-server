-- PowerFixer Server Schema
-- Consolidated initial schema

-- Enums for type safety
CREATE TYPE issue_provider AS ENUM ('github', 'linear');

CREATE TYPE issue_state AS ENUM (
    'untriaged',
    'triaged',
    'closed',
    'waiting_user',
    'waiting_warper',
    'agent_assigned',
    'agent_candidate',
    'dedupe_check'
);

CREATE TYPE agent_task_state AS ENUM (
    'queued',
    'in_progress',
    'succeeded',
    'failed'
);

CREATE TYPE agent_task_type AS ENUM (
    'fix',
    'dedupe',
    'triage'
);

CREATE TYPE local_agent_status AS ENUM (
    'running',
    'completed',
    'failed',
    'unknown'
);

-- Provider configuration: stores org/workspace info per provider
CREATE TABLE provider_configs (
    id SERIAL PRIMARY KEY,
    provider issue_provider NOT NULL,
    organization VARCHAR(255) NOT NULL,
    base_url TEXT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, organization)
);

-- Issues table: provider-agnostic issue tracking
CREATE TABLE issues (
    id SERIAL PRIMARY KEY,
    provider_config_id INTEGER NOT NULL REFERENCES provider_configs(id),
    external_id VARCHAR(255) NOT NULL,
    external_url TEXT,
    project VARCHAR(255) NOT NULL,
    title TEXT,
    state issue_state NOT NULL DEFAULT 'untriaged',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider_config_id, project, external_id)
);

-- Agent assignments: tracks cloud agent tasks assigned to issues
CREATE TABLE agent_assignments (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    agent_task_id VARCHAR(255),
    callback_token VARCHAR(255),
    session_url TEXT,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_by VARCHAR(100),
    task_state agent_task_state,
    task_type agent_task_type NOT NULL DEFAULT 'fix',
    branch_name VARCHAR(255),
    pr_url TEXT,
    summary TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Local agents: tracks locally-running oz agents
CREATE TABLE local_agents (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER REFERENCES issues(id) ON DELETE SET NULL,
    issue_number INTEGER,
    issue_title TEXT,
    conversation_id VARCHAR(255),
    prompt TEXT,
    started_at TIMESTAMPTZ NOT NULL,
    pid INTEGER NOT NULL,
    status local_agent_status NOT NULL DEFAULT 'running',
    log_path TEXT,
    callback_token TEXT,
    summary TEXT,
    branch_name TEXT,
    pr_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Triage agents: tracks batch triage tasks
CREATE TABLE triage_agents (
    id SERIAL PRIMARY KEY,
    task_id VARCHAR(255) NOT NULL UNIQUE,
    issue_ids INTEGER[] NOT NULL,
    issue_numbers INTEGER[] DEFAULT '{}',
    task_state agent_task_state,
    session_link TEXT,
    candidate_issue_ids INTEGER[] DEFAULT '{}',
    rejected_issues JSONB DEFAULT '[]',
    callback_token TEXT,
    started_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dedupe results: stores duplicate detection analysis
CREATE TABLE dedupe_results (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    issue_number INTEGER,
    canonical_issue_number INTEGER NOT NULL,
    duplicates JSONB NOT NULL,
    analysis_summary TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dedupe closures: tracks which duplicates were actually closed
CREATE TABLE dedupe_closures (
    id SERIAL PRIMARY KEY,
    dedupe_result_id INTEGER NOT NULL REFERENCES dedupe_results(id) ON DELETE CASCADE,
    closed_issue_number INTEGER NOT NULL,
    canonical_issue_number INTEGER NOT NULL,
    closed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_by VARCHAR(100)
);

-- Triage history: prevents re-triaging recently examined issues
CREATE TABLE triage_history (
    id SERIAL PRIMARY KEY,
    issue_number INTEGER NOT NULL,
    triage_agent_id INTEGER REFERENCES triage_agents(id) ON DELETE SET NULL,
    triaged_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    result VARCHAR(50) NOT NULL,
    rejection_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Inbox state: tracks read/archived status for TUI inbox
CREATE TABLE inbox_state (
    id SERIAL PRIMARY KEY,
    task_id VARCHAR(255) NOT NULL UNIQUE,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Action logs: audit trail of user actions
CREATE TABLE action_logs (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER REFERENCES issues(id) ON DELETE SET NULL,
    action VARCHAR(100) NOT NULL,
    details TEXT,
    logged_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_issues_provider_config ON issues(provider_config_id);
CREATE INDEX idx_issues_state ON issues(state);
CREATE INDEX idx_agent_assignments_issue ON agent_assignments(issue_id);
CREATE UNIQUE INDEX agent_assignments_callback_token_unique 
    ON agent_assignments(callback_token) 
    WHERE callback_token IS NOT NULL AND callback_token != '';
CREATE INDEX idx_provider_configs_default ON provider_configs(is_default) WHERE is_default = TRUE;
CREATE INDEX idx_local_agents_issue_number ON local_agents(issue_number);
CREATE INDEX idx_triage_agents_callback_token ON triage_agents(callback_token);
CREATE INDEX idx_dedupe_closures_result ON dedupe_closures(dedupe_result_id);
CREATE INDEX idx_dedupe_closures_issue ON dedupe_closures(closed_issue_number);
CREATE INDEX idx_triage_history_issue_number ON triage_history(issue_number);
CREATE INDEX idx_triage_history_triaged_at ON triage_history(triaged_at);
CREATE UNIQUE INDEX idx_triage_history_unique ON triage_history(issue_number, triage_agent_id);

-- Insert default GitHub provider config
INSERT INTO provider_configs (provider, organization, base_url, is_default)
VALUES ('github', 'example-org', 'https://github.com', TRUE);
