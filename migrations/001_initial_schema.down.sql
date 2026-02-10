-- Rollback PowerFixer Server Schema

DROP TABLE IF EXISTS action_logs;
DROP TABLE IF EXISTS inbox_state;
DROP TABLE IF EXISTS triage_history;
DROP TABLE IF EXISTS dedupe_closures;
DROP TABLE IF EXISTS dedupe_results;
DROP TABLE IF EXISTS triage_agents;
DROP TABLE IF EXISTS local_agents;
DROP TABLE IF EXISTS agent_assignments;
DROP TABLE IF EXISTS issues;
DROP TABLE IF EXISTS provider_configs;

DROP TYPE IF EXISTS local_agent_status;
DROP TYPE IF EXISTS agent_task_type;
DROP TYPE IF EXISTS agent_task_state;
DROP TYPE IF EXISTS issue_state;
DROP TYPE IF EXISTS issue_provider;
