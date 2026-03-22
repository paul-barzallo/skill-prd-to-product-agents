(function () {
  const snapshot = window.__REPORTING_SNAPSHOT__;

  function byId(id) {
    return document.getElementById(id);
  }

  function statusClass(value) {
    if (value === "stable" || value === "true" || value === true) return "ok";
    if (value === "watch" || value === "warn") return "warn";
    if (value === "attention" || value === "false" || value === false) return "danger";
    return "warn";
  }

  function metricCard(label, value, extra) {
    return `
      <article class="metric-card">
        <div class="label">${label}</div>
        <span class="value">${value}</span>
        ${extra ? `<div class="label">${extra}</div>` : ""}
      </article>
    `;
  }

  function badgeCard(label, enabled, meta) {
    return `
      <article class="badge-card">
        <div class="label">${label}</div>
        <span class="badge-pill ${statusClass(String(enabled))}">${enabled ? "enabled" : "degraded"}</span>
        <div class="label">${meta || ""}</div>
      </article>
    `;
  }

  function table(headers, rows) {
    if (!rows.length) {
      return '<p class="empty-state">No data available.</p>';
    }
    const head = headers.map((item) => `<th>${item.label}</th>`).join("");
    const body = rows.map((row) => `<tr>${headers.map((item) => `<td>${row[item.key] ?? ""}</td>`).join("")}</tr>`).join("");
    return `<table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
  }

  function renderReadiness() {
    const readiness = snapshot.readiness;
    byId("readiness-card").innerHTML = [
      metricCard("Current", readiness.current),
      metricCard("Missing for provisioned", readiness.missing_for_github_governance_provisioned.length),
      metricCard("Missing for production", readiness.missing_for_production_ready.length),
      metricCard("Visibility", snapshot.visibility_mode)
    ].join("");
  }

  function renderCapabilities() {
    const capabilities = snapshot.capabilities;
    const cards = [
      badgeCard("Git", capabilities.git.enabled, capabilities.git.mode),
      badgeCard("GitHub", capabilities.gh.enabled && capabilities.gh.authenticated, capabilities.gh.authenticated ? "authenticated" : "local-only"),
      badgeCard("SQLite", capabilities.sqlite.enabled, capabilities.sqlite.mode),
      badgeCard("markdownlint", capabilities.markdownlint.enabled, "documentation quality gate"),
      badgeCard("Local history", capabilities.local_history.enabled, capabilities.local_history.path),
      badgeCard("Reporting UI", capabilities.reporting.enabled, capabilities.reporting.visibility_mode_policy)
    ];
    byId("capability-badges").innerHTML = cards.join("");
  }

  function renderDelivery() {
    const delivery = snapshot.delivery_metrics;
    const total = Math.max(1, delivery.backlog + delivery.in_progress + delivery.blocked + delivery.done);
    const bars = [
      { label: "Backlog", value: delivery.backlog, className: "backlog" },
      { label: "In progress", value: delivery.in_progress, className: "progress" },
      { label: "Blocked", value: delivery.blocked, className: "blocked" },
      { label: "Done", value: delivery.done, className: "done" }
    ];
    byId("delivery-bars").innerHTML = bars.map((item) => `
      <div class="bar-row">
        <strong>${item.label}</strong>
        <div class="stack"><span class="segment ${item.className}" style="width:${(item.value / total) * 100}%"></span></div>
        <span>${item.value}</span>
      </div>
    `).join("");
    byId("delivery-table").innerHTML = table(
      [
        { key: "metric", label: "Metric" },
        { key: "value", label: "Value" },
        { key: "source", label: "Source" }
      ],
      [
        { metric: "Backlog", value: delivery.backlog, source: delivery.source },
        { metric: "In progress", value: delivery.in_progress, source: delivery.source },
        { metric: "Blocked", value: delivery.blocked, source: delivery.source },
        { metric: "Done", value: delivery.done, source: delivery.source },
        { metric: "Open PRs", value: delivery.open_prs, source: delivery.source }
      ]
    );
  }

  function renderTables() {
    byId("critical-findings").innerHTML = table(
      [
        { key: "id", label: "ID" },
        { key: "severity", label: "Severity" },
        { key: "status", label: "Status" },
        { key: "title", label: "Title" }
      ],
      snapshot.critical_findings
    );
    byId("pending-handoffs").innerHTML = table(
      [
        { key: "id", label: "ID" },
        { key: "from", label: "From" },
        { key: "to", label: "To" },
        { key: "status", label: "Status" }
      ],
      snapshot.pending_handoffs
    );
    byId("agent-health").innerHTML = table(
      [
        { key: "role", label: "Role" },
        { key: "health_status", label: "Health" },
        { key: "work_units_closed", label: "Work units" },
        { key: "pending_incoming_handoffs", label: "Incoming handoffs" },
        { key: "escalations", label: "Escalations" },
        { key: "validation_fail", label: "Validation fail" }
      ],
      snapshot.agent_health
    );
    byId("release-status").innerHTML = table(
      [
        { key: "id", label: "ID" },
        { key: "name", label: "Name" },
        { key: "status", label: "Status" },
        { key: "target_date", label: "Target date" },
        { key: "agent_role", label: "Agent role" }
      ],
      snapshot.releases
    );
  }

  function renderRisksAndNotes() {
    byId("risks-decisions").innerHTML = [
      metricCard("Open risks", snapshot.risks_and_decisions.open_risks),
      metricCard("Open questions", snapshot.risks_and_decisions.open_questions),
      metricCard("Closure coverage", `${snapshot.audit_metrics.closure_coverage_percent}%`, snapshot.audit_metrics.audit_source)
    ].join("");
    byId("visibility-notes").innerHTML = snapshot.visibility_notes.map((item) => `<li>${item}</li>`).join("");
  }

  function download(name, content, contentType) {
    const blob = new Blob([content], { type: contentType });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = name;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  function exportCsv() {
    const rows = [
      ["metric", "value"],
      ["generated_at_utc", snapshot.generated_at_utc],
      ["visibility_mode", snapshot.visibility_mode],
      ["readiness", snapshot.readiness.current],
      ["backlog", snapshot.delivery_metrics.backlog],
      ["in_progress", snapshot.delivery_metrics.in_progress],
      ["blocked", snapshot.delivery_metrics.blocked],
      ["done", snapshot.delivery_metrics.done],
      ["open_prs", snapshot.delivery_metrics.open_prs],
      ["closure_coverage_percent", snapshot.audit_metrics.closure_coverage_percent]
    ];
    download("report-summary.csv", rows.map((row) => row.join(",")).join("\n"), "text/csv;charset=utf-8");
  }

  function exportXlsx() {
    if (!window.XLSX) {
      alert("XLSX export is not available in this environment.");
      return;
    }
    const workbook = window.XLSX.utils.book_new();
    const summarySheet = window.XLSX.utils.json_to_sheet([{
      generated_at_utc: snapshot.generated_at_utc,
      visibility_mode: snapshot.visibility_mode,
      readiness: snapshot.readiness.current,
      backlog: snapshot.delivery_metrics.backlog,
      in_progress: snapshot.delivery_metrics.in_progress,
      blocked: snapshot.delivery_metrics.blocked,
      done: snapshot.delivery_metrics.done,
      open_prs: snapshot.delivery_metrics.open_prs
    }]);
    window.XLSX.utils.book_append_sheet(workbook, summarySheet, "Summary");
    window.XLSX.utils.book_append_sheet(workbook, window.XLSX.utils.json_to_sheet(snapshot.agent_health), "AgentHealth");
    const binary = window.XLSX.write(workbook, { type: "array", bookType: "xlsx" });
    download("report-pack.xlsx", binary, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
  }

  function bindActions() {
    byId("download-json").addEventListener("click", () => download("report-snapshot.json", JSON.stringify(snapshot, null, 2), "application/json;charset=utf-8"));
    byId("export-csv").addEventListener("click", exportCsv);
    byId("export-xlsx").addEventListener("click", exportXlsx);
    byId("print-dashboard").addEventListener("click", () => window.print());
  }

  function init() {
    byId("hero-summary").textContent = `Visibility: ${snapshot.visibility_mode}. Readiness: ${snapshot.readiness.current}. Delivery source: ${snapshot.delivery_source}.`;
    renderReadiness();
    renderCapabilities();
    renderDelivery();
    renderTables();
    renderRisksAndNotes();
    bindActions();
  }

  init();
})();
