//! Database query functions.
//!
//! This module contains all database operations for PowerFixer.

use chrono::Utc;

use super::models::*;
use super::DbPool;

// =============================================================================
// Provider Config
// =============================================================================

#[allow(dead_code)]
pub async fn get_default_provider_config(
    pool: &DbPool,
) -> Result<Option<ProviderConfig>, sqlx::Error> {
    sqlx::query_as::<_, ProviderConfig>(
        "SELECT * FROM provider_configs WHERE is_default = TRUE LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_provider_config(
    pool: &DbPool,
    provider: IssueProvider,
    organization: &str,
) -> Result<Option<ProviderConfig>, sqlx::Error> {
    sqlx::query_as::<_, ProviderConfig>(
        "SELECT * FROM provider_configs WHERE provider = $1 AND organization = $2",
    )
    .bind(provider)
    .bind(organization)
    .fetch_optional(pool)
    .await
}

// =============================================================================
// Issues
// =============================================================================

pub async fn upsert_issue(pool: &DbPool, issue: &NewIssue) -> Result<DbIssue, sqlx::Error> {
    sqlx::query_as::<_, DbIssue>(
        r#"
        INSERT INTO issues (provider_config_id, external_id, external_url, project, title, labels)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (provider_config_id, project, external_id) 
        DO UPDATE SET 
            external_url = EXCLUDED.external_url,
            title = EXCLUDED.title,
            labels = EXCLUDED.labels,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(issue.provider_config_id)
    .bind(&issue.external_id)
    .bind(&issue.external_url)
    .bind(&issue.project)
    .bind(&issue.title)
    .bind(&issue.labels)
    .fetch_one(pool)
    .await
}

pub async fn get_issue_by_id(pool: &DbPool, id: i32) -> Result<Option<DbIssue>, sqlx::Error> {
    sqlx::query_as::<_, DbIssue>("SELECT * FROM issues WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[allow(dead_code)]
pub async fn get_issue_by_external_id(
    pool: &DbPool,
    provider_config_id: i32,
    project: &str,
    external_id: &str,
) -> Result<Option<DbIssue>, sqlx::Error> {
    sqlx::query_as::<_, DbIssue>(
        "SELECT * FROM issues WHERE provider_config_id = $1 AND project = $2 AND external_id = $3",
    )
    .bind(provider_config_id)
    .bind(project)
    .bind(external_id)
    .fetch_optional(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_all_issues(pool: &DbPool) -> Result<Vec<DbIssue>, sqlx::Error> {
    sqlx::query_as::<_, DbIssue>("SELECT * FROM issues ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

#[allow(dead_code)]
pub async fn update_issue_labels(
    pool: &DbPool,
    issue_id: i32,
    labels: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE issues SET labels = $1, updated_at = NOW() WHERE id = $2")
        .bind(labels)
        .bind(issue_id)
        .execute(pool)
        .await?;
    Ok(())
}

// =============================================================================
// Unified Agents
// =============================================================================

pub async fn create_agent(pool: &DbPool, agent: &NewAgent) -> Result<DbAgent, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>(
        r#"
        INSERT INTO agents (agent_type, execution_mode, trigger_issue_id, task_id, callback_token, prompt, task_state, pid, log_path, trigger_source, triggered_by, started_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING *
        "#,
    )
    .bind(agent.agent_type)
    .bind(agent.execution_mode)
    .bind(agent.trigger_issue_id)
    .bind(&agent.task_id)
    .bind(&agent.callback_token)
    .bind(&agent.prompt)
    .bind(agent.task_state)
    .bind(agent.pid)
    .bind(&agent.log_path)
    .bind(agent.trigger_source)
    .bind(&agent.triggered_by)
    .bind(agent.started_at)
    .fetch_one(pool)
    .await
}

pub async fn get_agent_by_id(pool: &DbPool, id: i32) -> Result<Option<DbAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>("SELECT * FROM agents WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_agent_by_callback_token(
    pool: &DbPool,
    token: &str,
) -> Result<Option<DbAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>("SELECT * FROM agents WHERE callback_token = $1")
        .bind(token)
        .fetch_optional(pool)
        .await
}

pub async fn get_agent_by_task_id(
    pool: &DbPool,
    task_id: &str,
) -> Result<Option<DbAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>("SELECT * FROM agents WHERE task_id = $1")
        .bind(task_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_all_agents(pool: &DbPool) -> Result<Vec<DbAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>("SELECT * FROM agents ORDER BY started_at DESC")
        .fetch_all(pool)
        .await
}

#[allow(dead_code)]
pub async fn get_agents_by_type(
    pool: &DbPool,
    agent_type: AgentType,
) -> Result<Vec<DbAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>(
        "SELECT * FROM agents WHERE agent_type = $1 ORDER BY started_at DESC",
    )
    .bind(agent_type)
    .fetch_all(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_agents_by_execution_mode(
    pool: &DbPool,
    execution_mode: ExecutionMode,
) -> Result<Vec<DbAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>(
        "SELECT * FROM agents WHERE execution_mode = $1 ORDER BY started_at DESC",
    )
    .bind(execution_mode)
    .fetch_all(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_pending_agents(pool: &DbPool) -> Result<Vec<DbAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>(
        r#"
        SELECT * FROM agents 
        WHERE task_state IN ('queued', 'in_progress')
        ORDER BY started_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_pending_remote_agents(pool: &DbPool) -> Result<Vec<DbAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbAgent>(
        r#"
        SELECT * FROM agents 
        WHERE execution_mode = 'remote'
        AND task_id IS NOT NULL 
        AND task_state IN ('queued', 'in_progress')
        ORDER BY started_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

#[allow(dead_code)]
pub async fn update_agent_state(
    pool: &DbPool,
    id: i32,
    task_state: AgentTaskState,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE agents SET task_state = $1, updated_at = NOW() WHERE id = $2")
        .bind(task_state)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_agent_full(
    pool: &DbPool,
    id: i32,
    task_state: AgentTaskState,
    session_url: Option<&str>,
    branch_name: Option<&str>,
    pr_url: Option<&str>,
    summary: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE agents 
        SET task_state = $1, 
            session_url = COALESCE($2, session_url),
            branch_name = COALESCE($3, branch_name), 
            pr_url = COALESCE($4, pr_url), 
            summary = COALESCE($5, summary),
            updated_at = NOW() 
        WHERE id = $6
        "#,
    )
    .bind(task_state)
    .bind(session_url)
    .bind(branch_name)
    .bind(pr_url)
    .bind(summary)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn update_agent_task_id(
    pool: &DbPool,
    id: i32,
    task_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE agents SET task_id = $1, updated_at = NOW() WHERE id = $2")
        .bind(task_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn update_agent_session_url(
    pool: &DbPool,
    id: i32,
    session_url: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE agents SET session_url = $1, updated_at = NOW() WHERE id = $2")
        .bind(session_url)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_agent(pool: &DbPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM agents WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// =============================================================================
// Agent Status Updates
// =============================================================================

pub async fn create_agent_status_update(
    pool: &DbPool,
    update: &NewAgentStatusUpdate,
) -> Result<DbAgentStatusUpdate, sqlx::Error> {
    sqlx::query_as::<_, DbAgentStatusUpdate>(
        r#"
        INSERT INTO agent_status_updates (agent_id, state, message)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(update.agent_id)
    .bind(&update.state)
    .bind(&update.message)
    .fetch_one(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_agent_status_updates(
    pool: &DbPool,
    agent_id: i32,
) -> Result<Vec<DbAgentStatusUpdate>, sqlx::Error> {
    sqlx::query_as::<_, DbAgentStatusUpdate>(
        "SELECT * FROM agent_status_updates WHERE agent_id = $1 ORDER BY received_at DESC",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
}

// =============================================================================
// Issue Actions
// =============================================================================

pub async fn create_issue_action(
    pool: &DbPool,
    action: &NewIssueAction,
) -> Result<DbIssueAction, sqlx::Error> {
    sqlx::query_as::<_, DbIssueAction>(
        r#"
        INSERT INTO issue_actions (issue_id, action_type, details, performed_by)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(action.issue_id)
    .bind(action.action_type)
    .bind(&action.details)
    .bind(&action.performed_by)
    .fetch_one(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_issue_actions(
    pool: &DbPool,
    issue_id: i32,
) -> Result<Vec<DbIssueAction>, sqlx::Error> {
    sqlx::query_as::<_, DbIssueAction>(
        "SELECT * FROM issue_actions WHERE issue_id = $1 ORDER BY performed_at DESC",
    )
    .bind(issue_id)
    .fetch_all(pool)
    .await
}

// =============================================================================
// Triage Runs
// =============================================================================

pub async fn create_triage_run(
    pool: &DbPool,
    run: &NewTriageRun,
) -> Result<DbTriageRun, sqlx::Error> {
    sqlx::query_as::<_, DbTriageRun>(
        r#"
        INSERT INTO triage_runs (started_at, min_external_id, max_external_id)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(run.started_at)
    .bind(&run.min_external_id)
    .bind(&run.max_external_id)
    .fetch_one(pool)
    .await
}

pub async fn get_triage_run_by_id(
    pool: &DbPool,
    id: i32,
) -> Result<Option<DbTriageRun>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageRun>("SELECT * FROM triage_runs WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_all_triage_runs(pool: &DbPool) -> Result<Vec<DbTriageRun>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageRun>("SELECT * FROM triage_runs ORDER BY started_at DESC")
        .fetch_all(pool)
        .await
}

pub async fn get_recent_triage_runs(
    pool: &DbPool,
    limit: i64,
) -> Result<Vec<DbTriageRun>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageRun>("SELECT * FROM triage_runs ORDER BY started_at DESC LIMIT $1")
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn delete_triage_run(pool: &DbPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM triage_runs WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// =============================================================================
// Triage Run Agents
// =============================================================================

pub async fn create_triage_run_agent(
    pool: &DbPool,
    run_agent: &NewTriageRunAgent,
) -> Result<DbTriageRunAgent, sqlx::Error> {
    sqlx::query_as::<_, DbTriageRunAgent>(
        r#"
        INSERT INTO triage_run_agents (triage_run_id, agent_id, external_ids)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(run_agent.triage_run_id)
    .bind(run_agent.agent_id)
    .bind(&run_agent.external_ids)
    .fetch_one(pool)
    .await
}

pub async fn get_triage_run_agents(
    pool: &DbPool,
    triage_run_id: i32,
) -> Result<Vec<DbTriageRunAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageRunAgent>(
        "SELECT * FROM triage_run_agents WHERE triage_run_id = $1",
    )
    .bind(triage_run_id)
    .fetch_all(pool)
    .await
}

pub async fn get_triage_run_agent_by_agent_id(
    pool: &DbPool,
    agent_id: i32,
) -> Result<Option<DbTriageRunAgent>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageRunAgent>("SELECT * FROM triage_run_agents WHERE agent_id = $1")
        .bind(agent_id)
        .fetch_optional(pool)
        .await
}

// =============================================================================
// Triage Results
// =============================================================================

pub async fn create_triage_result(
    pool: &DbPool,
    result: &NewTriageResult,
) -> Result<DbTriageResult, sqlx::Error> {
    sqlx::query_as::<_, DbTriageResult>(
        r#"
        INSERT INTO triage_results (triage_run_id, agent_id, external_id, result, reason)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(result.triage_run_id)
    .bind(result.agent_id)
    .bind(&result.external_id)
    .bind(result.result)
    .bind(&result.reason)
    .fetch_one(pool)
    .await
}

pub async fn get_triage_results_for_run(
    pool: &DbPool,
    triage_run_id: i32,
) -> Result<Vec<DbTriageResult>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageResult>(
        "SELECT * FROM triage_results WHERE triage_run_id = $1 ORDER BY evaluated_at DESC",
    )
    .bind(triage_run_id)
    .fetch_all(pool)
    .await
}

pub async fn get_triage_results_for_agent(
    pool: &DbPool,
    agent_id: i32,
) -> Result<Vec<DbTriageResult>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageResult>(
        "SELECT * FROM triage_results WHERE agent_id = $1 ORDER BY evaluated_at DESC",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_triage_results_for_external_id(
    pool: &DbPool,
    external_id: &str,
) -> Result<Vec<DbTriageResult>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageResult>(
        "SELECT * FROM triage_results WHERE external_id = $1 ORDER BY evaluated_at DESC",
    )
    .bind(external_id)
    .fetch_all(pool)
    .await
}

pub async fn get_triage_candidates(pool: &DbPool) -> Result<Vec<DbTriageResult>, sqlx::Error> {
    sqlx::query_as::<_, DbTriageResult>(
        r#"
        SELECT DISTINCT ON (external_id) *
        FROM triage_results
        WHERE result = 'candidate'
        ORDER BY external_id, evaluated_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_recently_triaged_external_ids(
    pool: &DbPool,
    days: i32,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT external_id 
        FROM triage_results 
        WHERE evaluated_at > NOW() - INTERVAL '1 day' * $1
        "#,
    )
    .bind(days)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(n,)| n).collect())
}

pub async fn delete_triage_results_for_run(
    pool: &DbPool,
    triage_run_id: i32,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM triage_results WHERE triage_run_id = $1")
        .bind(triage_run_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// =============================================================================
// Dedupe Runs
// =============================================================================

pub async fn create_dedupe_run(
    pool: &DbPool,
    run: &NewDedupeRun,
) -> Result<DbDedupeRun, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeRun>(
        r#"
        INSERT INTO dedupe_runs (agent_id, canonical_issue_url, analysis_summary)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(run.agent_id)
    .bind(&run.canonical_issue_url)
    .bind(&run.analysis_summary)
    .fetch_one(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_dedupe_run_by_id(
    pool: &DbPool,
    id: i32,
) -> Result<Option<DbDedupeRun>, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeRun>("SELECT * FROM dedupe_runs WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_dedupe_run_by_agent_id(
    pool: &DbPool,
    agent_id: i32,
) -> Result<Option<DbDedupeRun>, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeRun>("SELECT * FROM dedupe_runs WHERE agent_id = $1")
        .bind(agent_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_dedupe_run_by_canonical_url(
    pool: &DbPool,
    canonical_issue_url: &str,
) -> Result<Option<DbDedupeRun>, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeRun>(
        "SELECT * FROM dedupe_runs WHERE canonical_issue_url = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(canonical_issue_url)
    .fetch_optional(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_all_dedupe_runs(pool: &DbPool) -> Result<Vec<DbDedupeRun>, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeRun>("SELECT * FROM dedupe_runs ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

#[allow(dead_code)]
pub async fn update_dedupe_run_summary(
    pool: &DbPool,
    id: i32,
    analysis_summary: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE dedupe_runs SET analysis_summary = $1 WHERE id = $2")
        .bind(analysis_summary)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// =============================================================================
// Dedupe Duplicates
// =============================================================================

pub async fn create_dedupe_duplicate(
    pool: &DbPool,
    duplicate: &NewDedupeDuplicate,
) -> Result<DbDedupeDuplicate, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeDuplicate>(
        r#"
        INSERT INTO dedupe_duplicates (dedupe_run_id, issue_url, confidence, reason)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(duplicate.dedupe_run_id)
    .bind(&duplicate.issue_url)
    .bind(duplicate.confidence)
    .bind(&duplicate.reason)
    .fetch_one(pool)
    .await
}

pub async fn get_dedupe_duplicates(
    pool: &DbPool,
    dedupe_run_id: i32,
) -> Result<Vec<DbDedupeDuplicate>, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeDuplicate>(
        "SELECT * FROM dedupe_duplicates WHERE dedupe_run_id = $1",
    )
    .bind(dedupe_run_id)
    .fetch_all(pool)
    .await
}

// =============================================================================
// Dedupe Closures
// =============================================================================

pub async fn create_dedupe_closure(
    pool: &DbPool,
    closure: &NewDedupeClosure,
) -> Result<DbDedupeClosure, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeClosure>(
        r#"
        INSERT INTO dedupe_closures (dedupe_run_id, closed_issue_url, closed_by)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(closure.dedupe_run_id)
    .bind(&closure.closed_issue_url)
    .bind(&closure.closed_by)
    .fetch_one(pool)
    .await
}

pub async fn get_dedupe_closures(
    pool: &DbPool,
    dedupe_run_id: i32,
) -> Result<Vec<DbDedupeClosure>, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeClosure>(
        "SELECT * FROM dedupe_closures WHERE dedupe_run_id = $1 ORDER BY closed_at DESC",
    )
    .bind(dedupe_run_id)
    .fetch_all(pool)
    .await
}

pub async fn mark_dedupe_run_addressed(
    pool: &DbPool,
    dedupe_run_id: i32,
) -> Result<DbDedupeRun, sqlx::Error> {
    sqlx::query_as::<_, DbDedupeRun>(
        r#"
        UPDATE dedupe_runs 
        SET is_addressed = TRUE, addressed_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(dedupe_run_id)
    .fetch_one(pool)
    .await
}

// =============================================================================
// Agent Inbox State
// =============================================================================

pub async fn upsert_agent_inbox_state(
    pool: &DbPool,
    agent_id: i32,
    is_archived: bool,
) -> Result<DbAgentInboxState, sqlx::Error> {
    sqlx::query_as::<_, DbAgentInboxState>(
        r#"
        INSERT INTO agent_inbox_state (agent_id, is_archived)
        VALUES ($1, $2)
        ON CONFLICT (agent_id) 
        DO UPDATE SET is_archived = $2, updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(agent_id)
    .bind(is_archived)
    .fetch_one(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_agent_inbox_state(
    pool: &DbPool,
    agent_id: i32,
) -> Result<Option<DbAgentInboxState>, sqlx::Error> {
    sqlx::query_as::<_, DbAgentInboxState>("SELECT * FROM agent_inbox_state WHERE agent_id = $1")
        .bind(agent_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_all_agent_inbox_states(
    pool: &DbPool,
) -> Result<Vec<DbAgentInboxState>, sqlx::Error> {
    sqlx::query_as::<_, DbAgentInboxState>("SELECT * FROM agent_inbox_state")
        .fetch_all(pool)
        .await
}

// =============================================================================
// Issue Inbox State
// =============================================================================

#[allow(dead_code)]
pub async fn upsert_issue_inbox_state(
    pool: &DbPool,
    issue_id: i32,
    is_archived: bool,
) -> Result<DbIssueInboxState, sqlx::Error> {
    sqlx::query_as::<_, DbIssueInboxState>(
        r#"
        INSERT INTO issue_inbox_state (issue_id, is_archived)
        VALUES ($1, $2)
        ON CONFLICT (issue_id) 
        DO UPDATE SET is_archived = $2, updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(issue_id)
    .bind(is_archived)
    .fetch_one(pool)
    .await
}

#[allow(dead_code)]
pub async fn get_issue_inbox_state(
    pool: &DbPool,
    issue_id: i32,
) -> Result<Option<DbIssueInboxState>, sqlx::Error> {
    sqlx::query_as::<_, DbIssueInboxState>("SELECT * FROM issue_inbox_state WHERE issue_id = $1")
        .bind(issue_id)
        .fetch_optional(pool)
        .await
}

#[allow(dead_code)]
pub async fn get_all_issue_inbox_states(
    pool: &DbPool,
) -> Result<Vec<DbIssueInboxState>, sqlx::Error> {
    sqlx::query_as::<_, DbIssueInboxState>("SELECT * FROM issue_inbox_state")
        .fetch_all(pool)
        .await
}

// =============================================================================
// Aggregate Queries
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct TriageRunSummary {
    pub run_id: i32,
    pub started_at: chrono::DateTime<Utc>,
    pub min_external_id: String,
    pub max_external_id: String,
    pub agent_count: i64,
    pub candidates_count: i64,
    pub rejected_count: i64,
    pub is_complete: bool,
}

pub async fn get_triage_run_summaries(
    pool: &DbPool,
    limit: i64,
) -> Result<Vec<TriageRunSummary>, sqlx::Error> {
    let runs = get_recent_triage_runs(pool, limit).await?;
    let mut summaries = Vec::new();

    for run in runs {
        let agents = get_triage_run_agents(pool, run.id).await?;
        let results = get_triage_results_for_run(pool, run.id).await?;

        let candidates_count = results
            .iter()
            .filter(|r| r.result == TriageResultType::Candidate)
            .count() as i64;
        let rejected_count = results
            .iter()
            .filter(|r| r.result == TriageResultType::Rejected)
            .count() as i64;

        let agent_ids: Vec<i32> = agents.iter().map(|a| a.agent_id).collect();
        let mut is_complete = !agent_ids.is_empty();
        for agent_id in &agent_ids {
            if let Some(agent) = get_agent_by_id(pool, *agent_id).await? {
                if !agent.task_state.is_terminal() {
                    is_complete = false;
                    break;
                }
            }
        }

        summaries.push(TriageRunSummary {
            run_id: run.id,
            started_at: run.started_at,
            min_external_id: run.min_external_id.clone(),
            max_external_id: run.max_external_id.clone(),
            agent_count: agents.len() as i64,
            candidates_count,
            rejected_count,
            is_complete,
        });
    }

    Ok(summaries)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TriageCoverage {
    pub examined_count: i64,
    pub candidates_count: i64,
    pub rejected_count: i64,
    pub agent_assigned_count: i64,
    pub min_triaged: Option<String>,
    pub max_triaged: Option<String>,
}

pub async fn get_triage_coverage(pool: &DbPool) -> Result<TriageCoverage, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64, i64, i64, Option<String>, Option<String>)>(
        r#"
        SELECT 
            COUNT(DISTINCT external_id) as examined_count,
            COUNT(DISTINCT CASE WHEN result = 'candidate' THEN external_id END) as candidates_count,
            COUNT(DISTINCT CASE WHEN result = 'rejected' THEN external_id END) as rejected_count,
            MIN(external_id) as min_triaged,
            MAX(external_id) as max_triaged
        FROM triage_results
        "#,
    )
    .fetch_one(pool)
    .await?;

    let agent_assigned = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT trigger_issue_id) FROM agents WHERE agent_type = 'fix' AND trigger_issue_id IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok(TriageCoverage {
        examined_count: row.0,
        candidates_count: row.1,
        rejected_count: row.2,
        agent_assigned_count: agent_assigned,
        min_triaged: row.3,
        max_triaged: row.4,
    })
}
