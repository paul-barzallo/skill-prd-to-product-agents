PRAGMA foreign_keys = ON;

-- ===================================================================
-- Schema v5 — Pure Audit Ledger
-- ===================================================================
-- Design: SQLite is a PASSIVE AUDIT LEDGER only. Agents never read
-- or write to this database. All writes happen automatically as
-- side-effects of file-first scripts (try_audit_activity in
-- _common-ops). Domain data lives exclusively in canonical docs
-- under docs/project/*.
--
-- v5 changes:
--   - Removed all operational/mutable tables (handoffs, findings,
--     releases, projects, environment_status, handoff_errors)
--   - Removed project_id from all tables
--   - Removed deprecated views (v_pending_handoffs, v_open_findings,
--     v_release_readiness, v_environment_health)
--   - Retained only append-only audit tables + sync tracking
-- ===================================================================

CREATE TABLE IF NOT EXISTS schema_version (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  description TEXT
);
INSERT OR IGNORE INTO schema_version (version, description) VALUES (1, 'Initial schema — bootstrap');
INSERT OR IGNORE INTO schema_version (version, description) VALUES (2, 'Add indices, metrics table, schema versioning');
INSERT OR IGNORE INTO schema_version (version, description) VALUES (3, 'Audit Ledger model — remove domain tables, add agent_activity_log');
INSERT OR IGNORE INTO schema_version (version, description) VALUES (4, 'Governed ledger — append-only triggers, handoff claim-lock, state machine');
INSERT OR IGNORE INTO schema_version (version, description) VALUES (5, 'Pure audit ledger — remove all operational tables');

-- ===================================================================
-- Artifact tracking — file-level checksums for sync
-- ===================================================================

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  artifact_type TEXT NOT NULL CHECK (artifact_type IN ('vision','scope','release','release-tracker','backlog','refined-stories','acceptance-criteria','gate','risk','handoff','finding','context-summary','ux','architecture','decision','report','other')),
  title TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL CHECK (status IN ('draft','proposed','approved','obsolete','removed')),
  owner_roles TEXT NOT NULL,
  checksum TEXT,
  last_synced_by_role TEXT CHECK (last_synced_by_role IN ('pm-orchestrator','product-owner','ux-designer','software-architect','tech-lead','backend-developer','frontend-developer','qa-lead','devops-release-engineer')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ===================================================================
-- Central audit log — every agent action in one place (APPEND-ONLY)
-- ===================================================================

CREATE TABLE IF NOT EXISTS agent_activity_log (
  id TEXT PRIMARY KEY,
  agent_role TEXT NOT NULL CHECK (agent_role IN ('pm-orchestrator','product-owner','ux-designer','software-architect','tech-lead','backend-developer','frontend-developer','qa-lead','devops-release-engineer')),
  activity_type TEXT NOT NULL CHECK (activity_type IN (
    'handoff_created','handoff_accepted','handoff_rejected','handoff_completed',
    'finding_reported','finding_triaged','finding_resolved',
    'gate_checked','release_checked','security_checked',
    'artifact_synced','artifact_created','artifact_updated',
    'review_recorded','env_event_logged',
    'run_started','run_completed','run_failed',
    'doc_created','doc_updated','context_injected',
    'bootstrap_completed','release_approved','release_deployed',
    'escalation','rework_requested','other'
  )),
  entity_type TEXT CHECK (entity_type IN ('artifact','finding','handoff','gate','release','environment','agent_run','milestone','security','review','other')),
  entity_ref TEXT,
  summary TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER IF NOT EXISTS trg_activity_log_no_update
  BEFORE UPDATE ON agent_activity_log
  BEGIN SELECT RAISE(ABORT, 'agent_activity_log is append-only: UPDATE not allowed'); END;

CREATE TRIGGER IF NOT EXISTS trg_activity_log_no_delete
  BEFORE DELETE ON agent_activity_log
  BEGIN SELECT RAISE(ABORT, 'agent_activity_log is append-only: DELETE not allowed'); END;

-- ===================================================================
-- Agent run tracking (APPEND-ONLY)
-- ===================================================================

CREATE TABLE IF NOT EXISTS agent_runs (
  id TEXT PRIMARY KEY,
  agent_role TEXT NOT NULL CHECK (agent_role IN ('pm-orchestrator','product-owner','ux-designer','software-architect','tech-lead','backend-developer','frontend-developer','qa-lead','devops-release-engineer')),
  workflow_name TEXT,
  input_ref TEXT,
  output_ref TEXT,
  status TEXT NOT NULL CHECK (status IN ('started','completed','failed','blocked')),
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  ended_at TEXT,
  summary TEXT
);

-- ===================================================================
-- Quality & security gates (audit of checks performed — APPEND-ONLY)
-- ===================================================================

CREATE TABLE IF NOT EXISTS gate_checks (
  id TEXT PRIMARY KEY,
  gate_ref TEXT NOT NULL,
  checked_by_role TEXT NOT NULL CHECK (checked_by_role IN ('product-owner','tech-lead','qa-lead','devops-release-engineer','pm-orchestrator')),
  result TEXT NOT NULL CHECK (result IN ('pass','fail','blocked','waived')),
  summary TEXT,
  checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER IF NOT EXISTS trg_gate_checks_no_update
  BEFORE UPDATE ON gate_checks
  BEGIN SELECT RAISE(ABORT, 'gate_checks is append-only: UPDATE not allowed'); END;

CREATE TRIGGER IF NOT EXISTS trg_gate_checks_no_delete
  BEFORE DELETE ON gate_checks
  BEGIN SELECT RAISE(ABORT, 'gate_checks is append-only: DELETE not allowed'); END;

CREATE TABLE IF NOT EXISTS security_checks (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL CHECK (entity_type IN ('story','release','architecture','environment','artifact')),
  entity_id TEXT NOT NULL,
  executed_by_role TEXT NOT NULL CHECK (executed_by_role IN ('qa-lead','tech-lead','devops-release-engineer')),
  check_name TEXT NOT NULL,
  result TEXT NOT NULL CHECK (result IN ('pass','fail','warning')),
  severity TEXT CHECK (severity IN ('low','medium','high','critical')),
  notes TEXT,
  checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER IF NOT EXISTS trg_security_checks_no_update
  BEFORE UPDATE ON security_checks
  BEGIN SELECT RAISE(ABORT, 'security_checks is append-only: UPDATE not allowed'); END;

CREATE TRIGGER IF NOT EXISTS trg_security_checks_no_delete
  BEFORE DELETE ON security_checks
  BEGIN SELECT RAISE(ABORT, 'security_checks is append-only: DELETE not allowed'); END;

CREATE TABLE IF NOT EXISTS release_checks (
  id TEXT PRIMARY KEY,
  release_ref TEXT NOT NULL,
  check_name TEXT NOT NULL,
  result TEXT NOT NULL CHECK (result IN ('pass','fail','blocked')),
  checked_by_role TEXT NOT NULL CHECK (checked_by_role IN ('product-owner','tech-lead','qa-lead','devops-release-engineer','pm-orchestrator')),
  notes TEXT,
  checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER IF NOT EXISTS trg_release_checks_no_update
  BEFORE UPDATE ON release_checks
  BEGIN SELECT RAISE(ABORT, 'release_checks is append-only: UPDATE not allowed'); END;

CREATE TRIGGER IF NOT EXISTS trg_release_checks_no_delete
  BEFORE DELETE ON release_checks
  BEGIN SELECT RAISE(ABORT, 'release_checks is append-only: DELETE not allowed'); END;

-- ===================================================================
-- Client reviews (APPEND-ONLY)
-- ===================================================================

CREATE TABLE IF NOT EXISTS client_reviews (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL CHECK (entity_type IN ('story','release','ux-artifact','milestone','artifact')),
  entity_id TEXT NOT NULL,
  recorded_by_role TEXT NOT NULL CHECK (recorded_by_role IN ('pm-orchestrator','product-owner','ux-designer','software-architect','tech-lead','backend-developer','frontend-developer','qa-lead','devops-release-engineer')),
  reviewer TEXT,
  result TEXT NOT NULL CHECK (result IN ('approved','changes_requested','rejected','not_applicable')),
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ===================================================================
-- Environment events (APPEND-ONLY log, not mutable status)
-- ===================================================================

CREATE TABLE IF NOT EXISTS environment_events (
  id TEXT PRIMARY KEY,
  env_name TEXT NOT NULL CHECK (env_name IN ('dev','qa','staging','prod')),
  event_type TEXT NOT NULL CHECK (event_type IN ('deploy_started','deploy_finished','deploy_failed','health_degraded','health_restored','rollback','incident_detected')),
  reported_by_role TEXT NOT NULL CHECK (reported_by_role IN ('pm-orchestrator','product-owner','ux-designer','software-architect','tech-lead','backend-developer','frontend-developer','qa-lead','devops-release-engineer')),
  build_version TEXT,
  severity TEXT CHECK (severity IN ('low','medium','high','critical')),
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ===================================================================
-- Milestone reports (APPEND-ONLY)
-- ===================================================================

CREATE TABLE IF NOT EXISTS milestone_reports (
  id TEXT PRIMARY KEY,
  milestone_ref TEXT NOT NULL,
  author_role TEXT NOT NULL CHECK (author_role IN ('pm-orchestrator','product-owner','ux-designer','software-architect','tech-lead','qa-lead','devops-release-engineer')),
  report_path TEXT,
  status_summary TEXT NOT NULL,
  risks_summary TEXT,
  next_step TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ===================================================================
-- Sync auditing
-- ===================================================================

CREATE TABLE IF NOT EXISTS sync_runs (
  id TEXT PRIMARY KEY,
  triggered_by_role TEXT NOT NULL CHECK (triggered_by_role IN ('pm-orchestrator','product-owner','ux-designer','software-architect','tech-lead','backend-developer','frontend-developer','qa-lead','devops-release-engineer')),
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  finished_at TEXT,
  result TEXT NOT NULL CHECK (result IN ('started','completed','failed','partial')),
  processed_artifacts INTEGER DEFAULT 0,
  changed_artifacts INTEGER DEFAULT 0,
  failures_count INTEGER DEFAULT 0,
  notes TEXT
);

CREATE TABLE IF NOT EXISTS sync_failures (
  id TEXT PRIMARY KEY,
  sync_run_id TEXT REFERENCES sync_runs(id),
  artifact_path TEXT NOT NULL,
  error_type TEXT NOT NULL CHECK (error_type IN ('parse_error','schema_validation','missing_reference','sql_error','checksum_error')),
  technical_detail TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open','resolved')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  resolved_at TEXT
);

-- ===================================================================
-- Operational metrics
-- ===================================================================

CREATE TABLE IF NOT EXISTS metrics (
  id TEXT PRIMARY KEY,
  metric_name TEXT NOT NULL,
  metric_value REAL NOT NULL,
  dimension TEXT,
  measured_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  source_role TEXT CHECK (source_role IN ('pm-orchestrator','product-owner','ux-designer','software-architect','tech-lead','backend-developer','frontend-developer','qa-lead','devops-release-engineer')),
  notes TEXT
);

-- ===================================================================
-- Views
-- ===================================================================

CREATE VIEW IF NOT EXISTS v_recent_activity AS
  SELECT id, agent_role, activity_type, entity_type,
         entity_ref, summary, created_at
  FROM agent_activity_log
  ORDER BY created_at DESC LIMIT 50;

CREATE VIEW IF NOT EXISTS v_agent_activity_summary AS
  SELECT agent_role,
         DATE(created_at) AS activity_date,
         activity_type,
         COUNT(*) AS event_count
  FROM agent_activity_log
  GROUP BY agent_role, DATE(created_at), activity_type
  ORDER BY DATE(created_at) DESC, agent_role;

-- ===================================================================
-- Performance indices
-- ===================================================================

CREATE INDEX IF NOT EXISTS idx_activity_agent_time ON agent_activity_log(agent_role, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_activity_type ON agent_activity_log(activity_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_activity_entity ON agent_activity_log(entity_type, entity_ref);
CREATE INDEX IF NOT EXISTS idx_artifacts_path ON artifacts(path);
CREATE INDEX IF NOT EXISTS idx_artifacts_type ON artifacts(artifact_type);
CREATE INDEX IF NOT EXISTS idx_sync_runs_started ON sync_runs(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_sync_failures_status ON sync_failures(status);
CREATE INDEX IF NOT EXISTS idx_gate_checks_ref ON gate_checks(gate_ref);
CREATE INDEX IF NOT EXISTS idx_env_events_env_time ON environment_events(env_name, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_release_checks_release ON release_checks(release_ref);
CREATE INDEX IF NOT EXISTS idx_agent_runs_role ON agent_runs(agent_role, status);
CREATE INDEX IF NOT EXISTS idx_security_checks_entity ON security_checks(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_metrics_name_time ON metrics(metric_name, measured_at DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_dimension ON metrics(dimension);
