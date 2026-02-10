-- Revert schema redesign (restore original schema would require re-running 001)
DROP TABLE IF EXISTS issue_inbox_state CASCADE;
DROP TABLE IF EXISTS agent_inbox_state CASCADE;
DROP TABLE IF EXISTS dedupe_closures CASCADE;
DROP TABLE IF EXISTS dedupe_duplicates CASCADE;
DROP TABLE IF EXISTS dedupe_runs CASCADE;
DROP TABLE IF EXISTS triage_results CASCADE;
DROP TABLE IF EXISTS triage_run_agents CASCADE;
DROP TABLE IF EXISTS triage_runs CASCADE;
DROP TABLE IF EXISTS issue_actions CASCADE;
DROP TABLE IF EXISTS agent_status_updates CASCADE;
DROP TABLE IF EXISTS agents CASCADE;
DROP TABLE IF EXISTS issues CASCADE;

DROP TYPE IF EXISTS issue_action_type CASCADE;
DROP TYPE IF EXISTS triage_result_type CASCADE;
DROP TYPE IF EXISTS agent_task_state CASCADE;
DROP TYPE IF EXISTS execution_mode CASCADE;
DROP TYPE IF EXISTS agent_type CASCADE;
