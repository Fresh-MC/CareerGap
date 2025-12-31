const { invoke } = window.__TAURI__.core;

// State management
let policies = [];
let auditResults = {};
let filteredPolicies = [];
let detectedOS = null;
let groupedPolicies = {};

// UI Elements
let loadPoliciesBtn;
let auditAllBtn;
let remediateAllBtn;
let rollbackAllBtn;
let rollbackOneBtn;
let policiesContainer;
let loadingEl;
let searchInput;
let platformFilter;
let severityFilter;
let statusFilter;
let policyCountEl;
let complianceStatusEl;
let systemInfoEl;
let modal;
let modalClose;
let generateReportBtn;
let exportCsvBtn;
let importCsvBtn;
let reportModal;
let reportModalClose;
let reportPreviewFrame;
let exportPdfBtn;
let currentReportPath = null;

// Initialize app
window.addEventListener("DOMContentLoaded", async () => {
  initializeElements();
  initializeAIElements(); // Initialize AI-assisted feature elements
  attachEventListeners();
  await loadSystemInfo();
});

function initializeElements() {
  loadPoliciesBtn = document.getElementById("load-policies-btn");
  auditAllBtn = document.getElementById("audit-all-btn");
  remediateAllBtn = document.getElementById("remediate-all-btn");
  rollbackAllBtn = document.getElementById("rollbackAllBtn");
  rollbackOneBtn = document.getElementById("rollbackOneBtn");
  policiesContainer = document.getElementById("policies-container");
  loadingEl = document.getElementById("loading");
  searchInput = document.getElementById("search-input");
  platformFilter = document.getElementById("platform-filter");
  severityFilter = document.getElementById("severity-filter");
  statusFilter = document.getElementById("status-filter");
  policyCountEl = document.getElementById("policy-count");
  complianceStatusEl = document.getElementById("compliance-status");
  systemInfoEl = document.getElementById("system-info");
  modal = document.getElementById("policy-modal");
  modalClose = document.querySelector(".close");
  generateReportBtn = document.getElementById("generate-report-btn");
  exportCsvBtn = document.getElementById("export-csv-btn");
  importCsvBtn = document.getElementById("import-csv-btn");
  reportModal = document.getElementById("report-modal");
  reportModalClose = document.querySelector(".report-close");
  reportPreviewFrame = document.getElementById("report-preview-frame");
  exportPdfBtn = document.getElementById("export-pdf-btn");
}

function attachEventListeners() {
  loadPoliciesBtn.addEventListener("click", loadPolicies);
  auditAllBtn.addEventListener("click", auditAllPolicies);
  remediateAllBtn.addEventListener("click", remediateAllPolicies);
  rollbackAllBtn.addEventListener("click", rollbackAll);
  searchInput.addEventListener("input", applyFilters);
  platformFilter.addEventListener("change", applyFilters);
  severityFilter.addEventListener("change", applyFilters);
  statusFilter.addEventListener("change", applyFilters);
  modalClose.addEventListener("click", closeModal);
  if (reportModalClose) {
    reportModalClose.addEventListener("click", closeReportModal);
  }
  if (generateReportBtn) {
    generateReportBtn.addEventListener("click", generateReport);
  }
  if (exportCsvBtn) {
    exportCsvBtn.addEventListener("click", exportCsvReport);
  }
  if (importCsvBtn) {
    importCsvBtn.addEventListener("click", () => {
      window.location.href = "csv_import.html";
    });
  }
  if (exportPdfBtn) {
    exportPdfBtn.addEventListener("click", exportReportToPdf);
  }
  // USB Manager navigation
  const usbManagerBtn = document.getElementById("usb-manager-btn");
  if (usbManagerBtn) {
    usbManagerBtn.addEventListener("click", () => {
      window.location.href = "usb.html";
    });
  }
  window.addEventListener("click", (e) => {
    if (e.target === modal) closeModal();
    if (e.target === reportModal) closeReportModal();
  });
}

async function loadSystemInfo() {
  try {
    const info = await invoke("get_system_info");
    systemInfoEl.textContent = `System: ${info}`;
  } catch (error) {
    systemInfoEl.textContent = "System: Unknown";
    console.error("Failed to load system info:", error);
  }
}

async function loadPolicies() {
  showLoading(true);
  try {
    policies = await invoke("load_policies");
    auditResults = {};
    
    // Detect OS and auto-filter policies
    if (!detectedOS) {
      const sysInfo = await invoke("get_system_info");
      detectedOS = sysInfo.toLowerCase().includes("windows") ? "windows" : 
                   sysInfo.toLowerCase().includes("linux") ? "linux" : null;
    }
    
    // Filter policies by detected OS
    if (detectedOS) {
      policies = policies.filter(p => p.platform.toLowerCase() === detectedOS);
    }
    
    filteredPolicies = [...policies];
    groupPoliciesByHierarchy();
    renderPolicies();
    updatePolicyCount();
    auditAllBtn.disabled = false;
    remediateAllBtn.disabled = false;
    rollbackAllBtn.disabled = false;
    showNotification("Policies loaded successfully", "success");
  } catch (error) {
    showNotification(`Failed to load policies: ${error}`, "error");
    console.error("Error loading policies:", error);
  }
  showLoading(false);
}

async function auditAllPolicies() {
  showLoading(true);
  try {
    const results = await invoke("audit_all_policies");
    results.forEach(result => {
      auditResults[result.policy_id] = result;
    });
    renderPolicies();
    updateComplianceStatus();
    enableAIButtons(); // Enable AI-assisted features after audit
    showNotification("Audit completed", "success");
  } catch (error) {
    showNotification(`Audit failed: ${error}`, "error");
    console.error("Error auditing policies:", error);
  }
  showLoading(false);
}

async function auditPolicy(policyId) {
  showLoading(true);
  try {
    const result = await invoke("audit_policy", { policyId });
    auditResults[policyId] = result;
    renderPolicies();
    updateComplianceStatus();
    showNotification(`Audited: ${policyId}`, "success");
  } catch (error) {
    showNotification(`Audit failed: ${error}`, "error");
    console.error("Error auditing policy:", error);
  }
  showLoading(false);
}

async function remediateAllPolicies() {
  if (!confirm("Are you sure you want to remediate ALL policies? This will modify system settings.")) {
    return;
  }
  showLoading(true);
  try {
    const results = await invoke("remediate_all_policies");
    let successCount = 0;
    let rebootRequired = false;
    results.forEach(result => {
      if (result.success) successCount++;
      if (result.reboot_required) rebootRequired = true;
    });
    
    let message = `Remediation complete: ${successCount}/${results.length} policies fixed`;
    if (rebootRequired) {
      message += " (Reboot required for some changes)";
    }
    showNotification(message, "success");
    
    // Re-audit after remediation
    await auditAllPolicies();
  } catch (error) {
    showNotification(`Remediation failed: ${error}`, "error");
    console.error("Error remediating policies:", error);
  }
  showLoading(false);
}

async function remediatePolicy(policyId) {
  if (!confirm(`Are you sure you want to remediate policy ${policyId}? This will modify system settings.`)) {
    return;
  }
  showLoading(true);
  try {
    const result = await invoke("remediate_policy", { policyId });
    if (result.success) {
      let message = `Policy ${policyId} remediated successfully`;
      if (result.reboot_required) {
        message += " (Reboot required)";
      }
      showNotification(message, "success");
      // Re-audit this policy
      await auditPolicy(policyId);
    } else {
      showNotification(`Remediation failed: ${result.message}`, "error");
    }
  } catch (error) {
    showNotification(`Remediation failed: ${error}`, "error");
    console.error("Error remediating policy:", error);
  }
  showLoading(false);
}

function groupPoliciesByHierarchy() {
  groupedPolicies = {};
  
  filteredPolicies.forEach(policy => {
    const parts = policy.id.split('.');
    const section = parts[0]; // e.g., "A"
    const subsection = parts.length > 1 ? parts[0] + '.' + parts[1] : null; // e.g., "A.1"
    
    if (!groupedPolicies[section]) {
      groupedPolicies[section] = { subsections: {}, policies: [] };
    }
    
    if (subsection && parts.length > 2) {
      if (!groupedPolicies[section].subsections[subsection]) {
        groupedPolicies[section].subsections[subsection] = [];
      }
      groupedPolicies[section].subsections[subsection].push(policy);
    } else {
      groupedPolicies[section].policies.push(policy);
    }
  });
}

function applyFilters() {
  const searchTerm = searchInput.value.toLowerCase();
  const platform = platformFilter.value;
  const severity = severityFilter.value;
  const status = statusFilter.value;

  filteredPolicies = policies.filter(policy => {
    const matchesSearch = !searchTerm || 
      policy.id.toLowerCase().includes(searchTerm) ||
      policy.title.toLowerCase().includes(searchTerm) ||
      policy.description.toLowerCase().includes(searchTerm);
    
    const matchesPlatform = platform === "all" || policy.platform === platform;
    const matchesSeverity = severity === "all" || policy.severity === severity;
    
    let matchesStatus = true;
    if (status !== "all") {
      const audit = auditResults[policy.id];
      if (status === "compliant") matchesStatus = audit && audit.compliant;
      else if (status === "non-compliant") matchesStatus = audit && !audit.compliant;
      else if (status === "pending") matchesStatus = !audit;
    }

    return matchesSearch && matchesPlatform && matchesSeverity && matchesStatus;
  });

  groupPoliciesByHierarchy();
  renderPolicies();
}

function renderPolicyCard(policy) {
  const audit = auditResults[policy.id];
  const statusClass = audit ? (audit.compliant ? 'compliant' : 'non-compliant') : 'pending';
  const statusText = audit ? (audit.compliant ? '✓ Compliant' : '✗ Non-Compliant') : '⦿ Pending';
  const severityClass = `severity-${policy.severity.toLowerCase()}`;

  return `
    <div class="policy-card ${statusClass}">
      <div class="policy-header">
        <div class="policy-title-section">
          <h3>${policy.title}</h3>
          <span class="policy-id">${policy.id}</span>
        </div>
        <div class="policy-badges">
          <span class="badge badge-platform">${policy.platform}</span>
          <span class="badge ${severityClass}">${policy.severity}</span>
          <span class="badge badge-status ${statusClass}">${statusText}</span>
        </div>
      </div>
      <p class="policy-description">${policy.description}</p>
      ${audit ? `<p class="audit-message">${audit.message}</p>` : ''}
      <div class="policy-actions">
        <button class="btn btn-small btn-secondary" onclick="auditPolicy('${policy.id}')">Audit</button>
        <button class="btn btn-small btn-primary" onclick="remediatePolicy('${policy.id}')">Remediate</button>
        <button class="btn btn-small btn-info" onclick="showPolicyDetails('${policy.id}')">Details</button>
      </div>
    </div>
  `;
}

function renderPolicies() {
  if (filteredPolicies.length === 0) {
    policiesContainer.innerHTML = '<div class="empty-state"><p>No policies found</p></div>';
    return;
  }

  let html = renderBulkRemediationPanel();

  // Render grouped policies
  Object.keys(groupedPolicies).sort().forEach(section => {
    const sectionData = groupedPolicies[section];
    const subsectionKeys = Object.keys(sectionData.subsections).sort();
    
    html += `
      <div class="policy-section">
        <div class="section-header" onclick="toggleSection('${section}')">
          <span class="section-toggle" id="toggle-${section}">▼</span>
          <h2>Section ${section}</h2>
        </div>
        <div class="section-content" id="section-${section}">
    `;
    
    // Render subsections
    subsectionKeys.forEach(subsection => {
      const policies = sectionData.subsections[subsection];
      html += `
        <div class="policy-subsection">
          <div class="subsection-header" onclick="toggleSubsection('${subsection}')">
            <span class="subsection-toggle" id="toggle-${subsection}">▼</span>
            <h3>Subsection ${subsection}</h3>
          </div>
          <div class="subsection-content" id="subsection-${subsection}">
            ${policies.map(p => renderPolicyCard(p)).join('')}
          </div>
        </div>
      `;
    });
    
    // Render policies without subsection
    if (sectionData.policies.length > 0) {
      html += sectionData.policies.map(p => renderPolicyCard(p)).join('');
    }
    
    html += `</div></div>`;
  });

  policiesContainer.innerHTML = html;
}

function renderBulkRemediationPanel() {
  const severities = [...new Set(filteredPolicies.map(p => p.severity))];
  
  if (severities.length === 0) return '';
  
  return `
    <div class="bulk-remediation-panel">
      <h2>🔧 Bulk Remediation</h2>
      <div class="bulk-actions">
        ${severities.map(severity => {
          const count = filteredPolicies.filter(p => p.severity === severity).length;
          const severityClass = `severity-${severity.toLowerCase()}`;
          return `
            <button class="btn btn-small ${severityClass}" onclick="remediateBySeverity('${severity}')">
              Remediate all ${severity} policies (${count})
            </button>
          `;
        }).join('')}
      </div>
    </div>
  `;
}

async function remediateBySeverity(severity) {
  const policiesToRemediate = filteredPolicies.filter(p => p.severity === severity);
  
  if (!confirm(`Are you sure you want to remediate all ${policiesToRemediate.length} ${severity} policies? This will modify system settings.`)) {
    return;
  }
  
  showLoading(true);
  let successCount = 0;
  let failCount = 0;
  let rebootRequired = false;
  
  for (const policy of policiesToRemediate) {
    try {
      const result = await invoke("remediate_policy", { policyId: policy.id });
      if (result.success) {
        successCount++;
        if (result.reboot_required) rebootRequired = true;
      } else {
        failCount++;
      }
    } catch (error) {
      failCount++;
      console.error(`Failed to remediate ${policy.id}:`, error);
    }
  }
  
  let message = `Bulk remediation complete: ${successCount} succeeded, ${failCount} failed`;
  if (rebootRequired) message += " (Reboot required)";
  
  showNotification(message, successCount > 0 ? "success" : "error");
  await auditAllPolicies();
  showLoading(false);
}

function toggleSection(sectionId) {
  const content = document.getElementById(`section-${sectionId}`);
  const toggle = document.getElementById(`toggle-${sectionId}`);
  
  if (content.style.display === 'none') {
    content.style.display = 'block';
    toggle.textContent = '▼';
  } else {
    content.style.display = 'none';
    toggle.textContent = '▶';
  }
}

function toggleSubsection(subsectionId) {
  const content = document.getElementById(`subsection-${subsectionId}`);
  const toggle = document.getElementById(`toggle-${subsectionId}`);
  
  if (content.style.display === 'none') {
    content.style.display = 'block';
    toggle.textContent = '▼';
  } else {
    content.style.display = 'none';
    toggle.textContent = '▶';
  }
}

function showPolicyDetails(policyId) {
  const policy = policies.find(p => p.id === policyId);
  const audit = auditResults[policyId];
  
  if (!policy) return;

  const modalTitle = document.getElementById("modal-title");
  const modalBody = document.getElementById("modal-body");

  modalTitle.textContent = policy.title;
  modalBody.innerHTML = `
    <div class="policy-details">
      <div class="detail-row"><strong>ID:</strong> ${policy.id}</div>
      <div class="detail-row"><strong>Platform:</strong> ${policy.platform}</div>
      <div class="detail-row"><strong>Severity:</strong> ${policy.severity}</div>
      <div class="detail-row"><strong>Description:</strong> ${policy.description}</div>
      ${audit ? `
        <div class="detail-row"><strong>Status:</strong> ${audit.compliant ? 'Compliant ✓' : 'Non-Compliant ✗'}</div>
        <div class="detail-row"><strong>Message:</strong> ${audit.message}</div>
      ` : '<div class="detail-row"><strong>Status:</strong> Not audited yet</div>'}
      <div id="rollback-result" class="rollback-result"></div>
    </div>
  `;

  // Show rollback button and attach handler
  rollbackOneBtn.style.display = "block";
  rollbackOneBtn.onclick = () => rollbackPolicy(policy.id);

  modal.style.display = "block";
}

function closeModal() {
  modal.style.display = "none";
}

function updatePolicyCount() {
  policyCountEl.textContent = `${policies.length} policies loaded`;
}

function updateComplianceStatus() {
  const auditedCount = Object.keys(auditResults).length;
  if (auditedCount === 0) {
    complianceStatusEl.textContent = "";
    return;
  }

  const compliantCount = Object.values(auditResults).filter(r => r.compliant).length;
  const percentage = Math.round((compliantCount / auditedCount) * 100);
  const statusClass = percentage === 100 ? 'success' : percentage >= 50 ? 'warning' : 'error';
  
  complianceStatusEl.innerHTML = `<span class="compliance-${statusClass}">Compliance: ${compliantCount}/${auditedCount} (${percentage}%)</span>`;
}

function showLoading(show) {
  loadingEl.style.display = show ? "flex" : "none";
}

function showNotification(message, type = "info") {
  const notification = document.createElement("div");
  notification.className = `notification notification-${type}`;
  notification.textContent = message;
  document.body.appendChild(notification);
  
  setTimeout(() => {
    notification.classList.add("show");
  }, 10);
  
  setTimeout(() => {
    notification.classList.remove("show");
    setTimeout(() => notification.remove(), 300);
  }, 3000);
}

// Rollback functions
async function rollbackPolicy(policyId) {
  if (!confirm(`Are you sure you want to rollback policy ${policyId}? This will restore the previous state.`)) {
    return;
  }
  showLoading(true);
  try {
    const result = await invoke("rollback_policy", { policyId });
    const resultDiv = document.getElementById("rollback-result");
    
    if (result.success) {
      if (resultDiv) {
        resultDiv.innerHTML = `<p class="rollback-success">✓ ${result.message}</p>`;
      }
      showNotification(`Policy ${policyId} rolled back successfully`, "success");
      // Re-audit this policy
      await auditPolicy(policyId);
    } else {
      if (resultDiv) {
        resultDiv.innerHTML = `<p class="rollback-failed">✗ ${result.message}</p>`;
      }
      showNotification(`Rollback failed: ${result.message}`, "error");
    }
  } catch (error) {
    const resultDiv = document.getElementById("rollback-result");
    if (resultDiv) {
      resultDiv.innerHTML = `<p class="rollback-failed">✗ ${error}</p>`;
    }
    showNotification(`Rollback failed: ${error}`, "error");
    console.error("Error rolling back policy:", error);
  }
  showLoading(false);
}

async function rollbackAll() {
  if (!confirm("Are you sure you want to rollback all recent changes? This will restore previous states for all modified policies.")) {
    return;
  }
  showLoading(true);
  try {
    const results = await invoke("rollback_all");
    let successCount = 0;
    let failedPolicies = [];
    
    results.forEach(result => {
      if (result.success) {
        successCount++;
      } else {
        failedPolicies.push(result.policy_id);
      }
    });
    
    let message = `Rollback complete: ${successCount}/${results.length} policies restored`;
    if (failedPolicies.length > 0) {
      message += `\nFailed: ${failedPolicies.join(", ")}`;
    }
    
    // Show detailed modal with results
    const modalTitle = document.getElementById("modal-title");
    const modalBody = document.getElementById("modal-body");
    
    modalTitle.textContent = "Rollback Summary";
    modalBody.innerHTML = `
      <div class="rollback-summary">
        <p><strong>Total Policies:</strong> ${results.length}</p>
        <p><strong>Successfully Rolled Back:</strong> ${successCount}</p>
        <p><strong>Failed:</strong> ${failedPolicies.length}</p>
        <div class="rollback-details">
          ${results.map(r => `
            <div class="detail-row ${r.success ? 'rollback-success' : 'rollback-failed'}">
              <strong>${r.policy_id}:</strong> ${r.message}
            </div>
          `).join('')}
        </div>
      </div>
    `;
    rollbackOneBtn.style.display = "none";
    modal.style.display = "block";
    
    showNotification(message, successCount === results.length ? "success" : "warning");
    
    // Re-audit all policies
    await auditAllPolicies();
  } catch (error) {
    showNotification(`Rollback failed: ${error}`, "error");
    console.error("Error rolling back all policies:", error);
  }
  showLoading(false);
}

// Report generation helper functions
function extractComplianceStats(audited) {
  const total = audited.length;
  const pass = audited.filter(a => a.compliant).length;
  const fail = total - pass;
  return { total, pass, fail };
}

function extractPlatformScores(audited) {
  const windowsPolicies = audited.filter(a => {
    const policy = policies.find(p => p.id === a.policy_id);
    return policy && policy.platform.toLowerCase() === 'windows';
  });
  const linuxPolicies = audited.filter(a => {
    const policy = policies.find(p => p.id === a.policy_id);
    return policy && policy.platform.toLowerCase() === 'linux';
  });

  const windowsScore = windowsPolicies.length > 0
    ? (windowsPolicies.filter(a => a.compliant).length / windowsPolicies.length) * 100
    : 0;
  const linuxScore = linuxPolicies.length > 0
    ? (linuxPolicies.filter(a => a.compliant).length / linuxPolicies.length) * 100
    : 0;

  return { windowsScore, linuxScore };
}

function closeReportModal() {
  if (reportModal) {
    reportModal.style.display = "none";
  }
  if (reportPreviewFrame) {
    reportPreviewFrame.src = "";
  }
  currentReportPath = null;
}

async function generateReport() {
  // Check if we have audit results
  const auditedResults = Object.values(auditResults);
  if (auditedResults.length === 0) {
    showNotification("Please run an audit before generating a report", "warning");
    return;
  }

  showLoading(true);
  try {
    // Prepare policy reports from audit results
    const policyReports = auditedResults.map(audit => {
      const policy = policies.find(p => p.id === audit.policy_id);
      return {
        policy_id: audit.policy_id,
        title: policy ? policy.title : audit.policy_id,
        status: audit.compliant ? "pass" : "fail"
      };
    });

    // Extract compliance statistics
    const { total, pass, fail } = extractComplianceStats(auditedResults);
    const { windowsScore, linuxScore } = extractPlatformScores(auditedResults);

    // Get current timestamp
    const timestamp = new Date().toISOString();

    // Call backend to generate HTML report
    const htmlPath = await invoke("generate_html_report", {
      policies: policyReports,
      total,
      pass,
      fail,
      windowsScore,
      linuxScore,
      timestamp
    });

    // Open save dialog to let user choose where to save the report
    const defaultFilename = `nogap_report_${timestamp.replace(/[:.\-]/g, '_')}.html`;
    const savePath = await window.__TAURI__.dialog.save({
      defaultPath: defaultFilename,
      filters: [{
        name: 'HTML Files',
        extensions: ['html']
      }]
    });

    // Check if user cancelled
    if (!savePath) {
      showNotification("Report generation cancelled", "info");
      showLoading(false);
      return;
    }

    // Read temp file and write to chosen location
    const fileData = await invoke("read_binary_file", { path: htmlPath });
    await invoke("write_binary_file", { path: savePath, data: Array.from(fileData) });

    showNotification(`Report saved successfully to ${savePath}`, "success");
  } catch (error) {
    showNotification(`Failed to generate report: ${error}`, "error");
    console.error("Error generating report:", error);
  }
  showLoading(false);
}

async function exportReportToPdf() {
  if (!currentReportPath) {
    showNotification("No report available to export", "warning");
    return;
  }

  showLoading(true);
  try {
    // Call backend to prepare PDF export
    const htmlPath = await invoke("export_pdf", {
      htmlPath: currentReportPath
    });

    // Use Tauri's opener plugin to open the HTML file in default browser
    // This avoids file:// URL issues in WebView
    await window.__TAURI__.core.invoke('plugin:opener|open', {
      path: htmlPath
    });
    
    showNotification("Opening report in browser for PDF export...", "success");
  } catch (error) {
    showNotification(`Failed to export PDF: ${error}`, "error");
    console.error("Error exporting PDF:", error);
  }
  showLoading(false);
}

async function exportCsvReport() {
  // Check if we have audit results
  const auditedResults = Object.values(auditResults);
  if (auditedResults.length === 0) {
    showNotification("Please run an audit before exporting CSV", "warning");
    return;
  }

  showLoading(true);
  try {
    // Prepare policy reports from audit results
    const policyReports = auditedResults.map(audit => {
      const policy = policies.find(p => p.id === audit.policy_id);
      return {
        policy_id: audit.policy_id,
        title: policy ? policy.title : audit.policy_id,
        status: audit.compliant ? "pass" : "fail"
      };
    });

    // Extract compliance statistics
    const { total, pass, fail } = extractComplianceStats(auditedResults);

    // Get current timestamp
    const timestamp = new Date().toISOString();

    // Generate CSV in temp directory
    const tempPath = await invoke("generate_csv_report", {
      policies: policyReports,
      total,
      pass,
      fail,
      timestamp
    });

    // Open save dialog
    const defaultFilename = `nogap_report_${timestamp.replace(/[:.\-]/g, '_')}.csv`;
    const savePath = await window.__TAURI__.dialog.save({
      defaultPath: defaultFilename,
      filters: [{
        name: 'CSV Files',
        extensions: ['csv']
      }]
    });

    // Check if user cancelled
    if (!savePath) {
      showNotification("CSV export cancelled", "info");
      showLoading(false);
      return;
    }

    // Read temp file and write to chosen location
    const fileData = await invoke("read_binary_file", { path: tempPath });
    await invoke("write_binary_file", { path: savePath, data: Array.from(fileData) });

    showNotification("CSV report saved successfully", "success");
  } catch (error) {
    showNotification(`Failed to export CSV: ${error}`, "error");
    console.error("Error exporting CSV:", error);
  }
  showLoading(false);
}

// ============================================================
// AI-ASSISTED FEATURES (Non-Agentic, Read-Only, User-Controlled)
// ============================================================

// AI Feature Elements
let riskReportBtn;
let driftDetectBtn;
let recommendationsBtn;
let riskReportModal;
let driftModal;
let recommendationsModal;

// Initialize AI feature elements after DOM loads
function initializeAIElements() {
  riskReportBtn = document.getElementById("risk-report-btn");
  driftDetectBtn = document.getElementById("drift-detect-btn");
  recommendationsBtn = document.getElementById("recommendations-btn");
  riskReportModal = document.getElementById("risk-report-modal");
  driftModal = document.getElementById("drift-modal");
  recommendationsModal = document.getElementById("recommendations-modal");

  // Attach event listeners
  if (riskReportBtn) {
    riskReportBtn.addEventListener("click", showRiskReport);
  }
  if (driftDetectBtn) {
    driftDetectBtn.addEventListener("click", detectDrift);
  }
  if (recommendationsBtn) {
    recommendationsBtn.addEventListener("click", showRecommendationsModal);
  }
  
  // Modal close handlers
  const riskClose = document.querySelector(".risk-close");
  if (riskClose) {
    riskClose.addEventListener("click", () => riskReportModal.style.display = "none");
  }
  const driftClose = document.querySelector(".drift-close");
  if (driftClose) {
    driftClose.addEventListener("click", () => driftModal.style.display = "none");
  }
  const recommendationsClose = document.querySelector(".recommendations-close");
  if (recommendationsClose) {
    recommendationsClose.addEventListener("click", () => recommendationsModal.style.display = "none");
  }
  
  // Get recommendations button inside modal
  const getRecommendationsBtn = document.getElementById("get-recommendations-btn");
  if (getRecommendationsBtn) {
    getRecommendationsBtn.addEventListener("click", fetchRecommendations);
  }
}

// Enable AI buttons after audit
function enableAIButtons() {
  if (riskReportBtn) riskReportBtn.disabled = false;
  if (driftDetectBtn) driftDetectBtn.disabled = false;
  if (recommendationsBtn) recommendationsBtn.disabled = false;
}

// Risk Report Feature
async function showRiskReport() {
  showLoading(true);
  try {
    const report = await invoke("cmd_get_risk_report");
    
    // Update aggregate score
    const scoreEl = document.getElementById("aggregate-risk-score");
    const score = (report.aggregate_score * 100).toFixed(1);
    scoreEl.textContent = `${score}%`;
    scoreEl.className = "risk-score-value";
    if (report.aggregate_score < 0.3) scoreEl.classList.add("low");
    else if (report.aggregate_score < 0.6) scoreEl.classList.add("medium");
    else scoreEl.classList.add("high");
    
    // Update stats
    document.getElementById("total-policies-count").textContent = report.total_policies;
    document.getElementById("compliant-count").textContent = report.compliant_count;
    document.getElementById("non-compliant-count").textContent = report.non_compliant_count;
    
    // Render top risks
    const listEl = document.getElementById("top-risks-list");
    if (report.top_risks.length === 0) {
      listEl.innerHTML = '<p class="text-success">✅ No high-risk policies found. Your system is well-configured!</p>';
    } else {
      listEl.innerHTML = report.top_risks.map(risk => `
        <div class="risk-item ${risk.severity}">
          <div class="risk-item-header">
            <strong>${risk.title || risk.policy_id}</strong>
            <span class="risk-score-badge">${(risk.risk_score * 100).toFixed(0)}% risk</span>
          </div>
          <div class="text-muted">${risk.policy_id}</div>
          <div class="text-muted">Severity: ${risk.severity.toUpperCase()}</div>
        </div>
      `).join('');
    }
    
    riskReportModal.style.display = "block";
    showNotification("Risk report generated (AI-assisted)", "success");
  } catch (error) {
    showNotification(`Failed to generate risk report: ${error}`, "error");
    console.error("Risk report error:", error);
  }
  showLoading(false);
}

// Drift Detection Feature
async function detectDrift() {
  showLoading(true);
  try {
    const report = await invoke("cmd_detect_drift");
    
    // Update summary
    document.getElementById("drift-summary-text").textContent = report.summary;
    
    // Render drift events
    const listEl = document.getElementById("drift-events-list");
    if (!report.has_drift) {
      listEl.innerHTML = '<p class="text-success">✅ No compliance drift detected. All policies stable since last audit.</p>';
    } else {
      listEl.innerHTML = report.events.map(event => `
        <div class="drift-item">
          <div class="drift-item-header">
            <strong>${event.title || event.policy_id}</strong>
            <span class="text-danger">⚠️ REGRESSION</span>
          </div>
          <div class="text-muted">${event.policy_id}</div>
          <div>
            <span class="text-success">${event.previous_state}</span>
            → 
            <span class="text-danger">${event.current_state}</span>
          </div>
          <div class="text-muted">Detected: ${new Date(parseInt(event.timestamp) * 1000).toLocaleString()}</div>
        </div>
      `).join('');
    }
    
    driftModal.style.display = "block";
    showNotification(`Drift detection complete: ${report.events.length} regression(s) found (AI-assisted)`, 
                     report.has_drift ? "warning" : "success");
  } catch (error) {
    showNotification(`Failed to detect drift: ${error}`, "error");
    console.error("Drift detection error:", error);
  }
  showLoading(false);
}

// Recommendations Feature
function showRecommendationsModal() {
  recommendationsModal.style.display = "block";
}

async function fetchRecommendations() {
  const role = document.getElementById("role-input").value || "general";
  const environment = document.getElementById("environment-input").value;
  
  showLoading(true);
  try {
    const response = await invoke("cmd_get_recommendations", {
      role,
      environment,
      additionalContext: null
    });
    
    // Update context summary
    document.getElementById("recommendations-context").textContent = response.context_summary;
    
    // Render recommendations
    const listEl = document.getElementById("recommendations-list");
    if (response.recommendations.length === 0) {
      listEl.innerHTML = '<p class="text-muted">No specific recommendations for this context. Try adjusting the role or environment.</p>';
    } else {
      listEl.innerHTML = response.recommendations.map(rec => `
        <div class="recommendation-item" onclick="showPolicyDetails('${rec.policy_id}')">
          <div class="recommendation-header">
            <strong>${rec.title}</strong>
            <span class="severity-badge severity-${rec.severity}">${rec.severity.toUpperCase()}</span>
          </div>
          <div class="text-muted">${rec.policy_id}</div>
          <div>${rec.description}</div>
          <div class="relevance-score">
            Relevance: ${(rec.relevance_score * 100).toFixed(0)}% | ${rec.reason}
          </div>
        </div>
      `).join('');
    }
    
    showNotification(`Found ${response.recommendations.length} recommended policies (AI-assisted)`, "success");
  } catch (error) {
    showNotification(`Failed to get recommendations: ${error}`, "error");
    console.error("Recommendations error:", error);
  }
  showLoading(false);
}

// ============================================================
// AUTONOMOUS MONITORING - Sensor GUI Integration
// ============================================================

// Sensor UI Elements
let sensorEnabledToggle;
let sensorIntervalSelect;
let sensorStartBtn;
let sensorStopBtn;
let sensorScanNowBtn;
let sensorStatusBadge;
let sensorRunningStatus;
let sensorLastScan;
let sensorNextScan;
let sensorSummaryPanel;
let sensorPoliciesChecked;
let sensorCompliantCount;
let sensorNonCompliantCount;
let sensorDriftCount;
let viewSensorReportBtn;
let sensorReportModal;
let sensorReportClose;

// Initialize sensor UI elements
function initializeSensorElements() {
  sensorEnabledToggle = document.getElementById("sensor-enabled-toggle");
  sensorIntervalSelect = document.getElementById("sensor-interval");
  sensorStartBtn = document.getElementById("sensor-start-btn");
  sensorStopBtn = document.getElementById("sensor-stop-btn");
  sensorScanNowBtn = document.getElementById("sensor-scan-now-btn");
  sensorStatusBadge = document.getElementById("sensor-status-badge");
  sensorRunningStatus = document.getElementById("sensor-running-status");
  sensorLastScan = document.getElementById("sensor-last-scan");
  sensorNextScan = document.getElementById("sensor-next-scan");
  sensorSummaryPanel = document.getElementById("sensor-summary-panel");
  sensorPoliciesChecked = document.getElementById("sensor-policies-checked");
  sensorCompliantCount = document.getElementById("sensor-compliant-count");
  sensorNonCompliantCount = document.getElementById("sensor-non-compliant-count");
  sensorDriftCount = document.getElementById("sensor-drift-count");
  viewSensorReportBtn = document.getElementById("view-sensor-report-btn");
  sensorReportModal = document.getElementById("sensor-report-modal");
  sensorReportClose = document.querySelector(".sensor-report-close");
}

// Attach sensor event listeners
function attachSensorEventListeners() {
  if (sensorEnabledToggle) {
    sensorEnabledToggle.addEventListener("change", handleSensorToggle);
  }
  if (sensorIntervalSelect) {
    sensorIntervalSelect.addEventListener("change", handleIntervalChange);
  }
  if (sensorStartBtn) {
    sensorStartBtn.addEventListener("click", startSensor);
  }
  if (sensorStopBtn) {
    sensorStopBtn.addEventListener("click", stopSensor);
  }
  if (sensorScanNowBtn) {
    sensorScanNowBtn.addEventListener("click", runSensorScanNow);
  }
  if (viewSensorReportBtn) {
    viewSensorReportBtn.addEventListener("click", showSensorReport);
  }
  if (sensorReportClose) {
    sensorReportClose.addEventListener("click", closeSensorReportModal);
  }
  // Close modal on outside click
  window.addEventListener("click", (e) => {
    if (e.target === sensorReportModal) closeSensorReportModal();
  });
}

// Load initial sensor state
async function loadSensorState() {
  try {
    const state = await invoke("cmd_get_sensor_state");
    updateSensorUI(state);
  } catch (error) {
    console.error("Failed to load sensor state:", error);
    // Fail gracefully - just show default state
  }
}

// Update sensor UI based on state
function updateSensorUI(state) {
  // Update toggle
  if (sensorEnabledToggle) {
    sensorEnabledToggle.checked = state.enabled;
  }
  
  // Update interval
  if (sensorIntervalSelect) {
    sensorIntervalSelect.value = state.interval_hours.toString();
  }
  
  // Update status badge
  if (sensorStatusBadge) {
    if (state.is_running) {
      sensorStatusBadge.textContent = "Running";
      sensorStatusBadge.className = "sensor-badge running";
    } else if (state.enabled) {
      sensorStatusBadge.textContent = "Enabled";
      sensorStatusBadge.className = "sensor-badge active";
    } else {
      sensorStatusBadge.textContent = "Disabled";
      sensorStatusBadge.className = "sensor-badge";
    }
  }
  
  // Update running status
  if (sensorRunningStatus) {
    if (state.is_running) {
      sensorRunningStatus.textContent = "Running";
      sensorRunningStatus.className = "status-value running";
    } else {
      sensorRunningStatus.textContent = "Not Running";
      sensorRunningStatus.className = "status-value stopped";
    }
  }
  
  // Update timestamps
  if (sensorLastScan) {
    sensorLastScan.textContent = state.last_run || "Never";
  }
  if (sensorNextScan) {
    sensorNextScan.textContent = state.next_run || "--";
  }
  
  // Update buttons
  if (sensorStartBtn) {
    sensorStartBtn.disabled = !state.enabled || state.is_running;
  }
  if (sensorStopBtn) {
    sensorStopBtn.disabled = !state.is_running;
  }
  if (sensorScanNowBtn) {
    sensorScanNowBtn.disabled = !state.enabled;
  }
  
  // Update summary panel
  if (state.last_scan_summary && sensorSummaryPanel) {
    sensorSummaryPanel.style.display = "block";
    sensorPoliciesChecked.textContent = state.last_scan_summary.policies_checked;
    sensorCompliantCount.textContent = state.last_scan_summary.compliant;
    sensorNonCompliantCount.textContent = state.last_scan_summary.non_compliant;
    sensorDriftCount.textContent = state.last_scan_summary.drift_events;
  } else if (state.has_history && sensorSummaryPanel) {
    // Show panel but indicate we need to load history
    sensorSummaryPanel.style.display = "block";
    loadLastSensorSummary();
  }
}

// Load last sensor summary from history
async function loadLastSensorSummary() {
  try {
    const report = await invoke("cmd_get_last_sensor_report");
    if (report.summary) {
      sensorPoliciesChecked.textContent = report.summary.policies_checked;
      sensorCompliantCount.textContent = report.summary.compliant;
      sensorNonCompliantCount.textContent = report.summary.non_compliant;
      sensorDriftCount.textContent = report.summary.drift_events;
      
      if (report.timestamp && sensorLastScan) {
        sensorLastScan.textContent = report.timestamp;
      }
    }
  } catch (error) {
    console.error("Failed to load sensor summary:", error);
  }
}

// Handle sensor enable/disable toggle
async function handleSensorToggle() {
  const enabled = sensorEnabledToggle.checked;
  
  try {
    const state = await invoke("cmd_update_sensor_config", {
      config: { enabled, interval_hours: null }
    });
    updateSensorUI(state);
    showNotification(
      enabled ? "Autonomous monitoring enabled" : "Autonomous monitoring disabled",
      "success"
    );
  } catch (error) {
    showNotification(`Failed to update sensor config: ${error}`, "error");
    // Revert toggle
    sensorEnabledToggle.checked = !enabled;
  }
}

// Handle interval change
async function handleIntervalChange() {
  const interval = parseInt(sensorIntervalSelect.value);
  
  try {
    const state = await invoke("cmd_update_sensor_config", {
      config: { enabled: null, interval_hours: interval }
    });
    updateSensorUI(state);
    showNotification(`Scan interval updated to ${interval} hours`, "success");
  } catch (error) {
    showNotification(`Failed to update interval: ${error}`, "error");
  }
}

// Start the sensor
async function startSensor() {
  sensorStartBtn.disabled = true;
  
  try {
    const state = await invoke("cmd_start_sensor");
    updateSensorUI(state);
    showNotification("Autonomous sensor started", "success");
  } catch (error) {
    showNotification(`Failed to start sensor: ${error}`, "error");
    sensorStartBtn.disabled = false;
  }
}

// Stop the sensor
async function stopSensor() {
  sensorStopBtn.disabled = true;
  
  try {
    const state = await invoke("cmd_stop_sensor");
    updateSensorUI(state);
    showNotification("Autonomous sensor stopped", "success");
  } catch (error) {
    showNotification(`Failed to stop sensor: ${error}`, "error");
    sensorStopBtn.disabled = false;
  }
}

// Run sensor scan immediately
async function runSensorScanNow() {
  sensorScanNowBtn.disabled = true;
  showLoading(true);
  
  try {
    const summary = await invoke("cmd_run_sensor_scan_now");
    
    // Update summary display
    if (sensorSummaryPanel) {
      sensorSummaryPanel.style.display = "block";
      sensorPoliciesChecked.textContent = summary.policies_checked;
      sensorCompliantCount.textContent = summary.compliant;
      sensorNonCompliantCount.textContent = summary.non_compliant;
      sensorDriftCount.textContent = summary.drift_events;
    }
    
    // Refresh full state to get updated timestamps
    const state = await invoke("cmd_get_sensor_state");
    updateSensorUI(state);
    
    showNotification(
      `Autonomous scan complete: ${summary.policies_checked} policies, ${summary.drift_events} drift events`,
      "success"
    );
  } catch (error) {
    showNotification(`Scan failed: ${error}`, "error");
  }
  
  showLoading(false);
  sensorScanNowBtn.disabled = false;
}

// Show sensor report modal
async function showSensorReport() {
  showLoading(true);
  
  try {
    const report = await invoke("cmd_get_last_sensor_report");
    
    if (!report.timestamp) {
      showNotification("No autonomous scan reports available yet", "warning");
      showLoading(false);
      return;
    }
    
    // Update report modal content
    document.getElementById("report-scan-time").textContent = report.timestamp;
    
    if (report.summary) {
      document.getElementById("report-policies-total").textContent = report.summary.policies_checked;
      document.getElementById("report-compliant").textContent = report.summary.compliant;
      document.getElementById("report-non-compliant").textContent = report.summary.non_compliant;
    }
    
    // Render audit results
    const resultsEl = document.getElementById("sensor-report-results");
    if (report.audit_results.length === 0) {
      resultsEl.innerHTML = '<p class="text-muted">No audit results available.</p>';
    } else {
      resultsEl.innerHTML = report.audit_results.map(item => `
        <div class="sensor-report-item ${item.compliant ? 'compliant' : 'non-compliant'}">
          <span class="policy-id">${item.policy_id}</span>
          <span class="policy-message">${item.message}</span>
          <span class="status-icon">${item.compliant ? '✅' : '❌'}</span>
        </div>
      `).join('');
    }
    
    // Show modal
    sensorReportModal.style.display = "block";
    
  } catch (error) {
    showNotification(`Failed to load sensor report: ${error}`, "error");
  }
  
  showLoading(false);
}

// Close sensor report modal
function closeSensorReportModal() {
  if (sensorReportModal) {
    sensorReportModal.style.display = "none";
  }
}

// Initialize sensor on page load
async function initializeSensor() {
  initializeSensorElements();
  attachSensorEventListeners();
  await loadSensorState();
  
  // Refresh sensor state periodically (every 30 seconds)
  setInterval(async () => {
    try {
      const state = await invoke("cmd_get_sensor_state");
      updateSensorUI(state);
    } catch (error) {
      // Silently fail on periodic refresh
    }
  }, 30000);
}

// Make functions global for onclick handlers
window.auditPolicy = auditPolicy;
window.remediatePolicy = remediatePolicy;
window.showPolicyDetails = showPolicyDetails;
window.rollbackPolicy = rollbackPolicy;
window.rollbackAll = rollbackAll;
window.generateReport = generateReport;
window.exportCsvReport = exportCsvReport;
window.exportReportToPdf = exportReportToPdf;
window.remediateBySeverity = remediateBySeverity;
window.toggleSection = toggleSection;
window.toggleSubsection = toggleSubsection;
window.showRiskReport = showRiskReport;
window.detectDrift = detectDrift;
window.showRecommendationsModal = showRecommendationsModal;
window.fetchRecommendations = fetchRecommendations;
// Sensor functions
window.startSensor = startSensor;
window.stopSensor = stopSensor;
window.runSensorScanNow = runSensorScanNow;
window.showSensorReport = showSensorReport;
window.closeSensorReportModal = closeSensorReportModal;

// Initialize sensor when DOM is ready
document.addEventListener("DOMContentLoaded", () => {
  // Wait a bit for main initialization to complete
  setTimeout(initializeSensor, 100);
  setTimeout(initializePlanner, 150);
});

// ============================================================
// REMEDIATION PLANNER FUNCTIONS
// ============================================================

// Planner UI elements
let generatePlanBtn;
let clearPlanBtn;
let planGoalEl;
let planSnapshotTimeEl;
let planComplianceRateEl;
let planGeneratedTimeEl;
let planStepsCountEl;
let planDeferredCountEl;
let approvePlanBtn;
let approvalStatusEl;
let planStepsList;
let planDeferredList;
let plannerBadge;
let plannerStepModal;
let planSummaryCard;
let planStepsSection;
let planDeferredSection;
let planExcludedSection;
let planExcludedList;
let planModifiedNotice;
let addPolicyBtn;
let addPolicyModal;
let planAckModal;

// Current plan state
let currentPlan = null;
let eligiblePolicies = [];

// Initialize planner elements
function initializePlannerElements() {
  generatePlanBtn = document.getElementById("generate-plan-btn");
  clearPlanBtn = document.getElementById("clear-plan-btn");
  planGoalEl = document.getElementById("plan-goal");
  planSnapshotTimeEl = document.getElementById("plan-snapshot-time");
  planComplianceRateEl = document.getElementById("plan-compliance-rate");
  planGeneratedTimeEl = document.getElementById("plan-generated-time");
  planStepsCountEl = document.getElementById("plan-steps-count");
  planDeferredCountEl = document.getElementById("plan-deferred-count");
  approvePlanBtn = document.getElementById("approve-plan-btn");
  approvalStatusEl = document.getElementById("plan-approval-status");
  planStepsList = document.getElementById("plan-steps-list");
  planDeferredList = document.getElementById("plan-deferred-list");
  plannerBadge = document.getElementById("planner-status-badge");
  plannerStepModal = document.getElementById("planner-step-modal");
  planSummaryCard = document.getElementById("plan-summary-card");
  planStepsSection = document.getElementById("plan-steps-section");
  planDeferredSection = document.getElementById("plan-deferred-section");
  planExcludedSection = document.getElementById("plan-excluded-section");
  planExcludedList = document.getElementById("plan-excluded-list");
  planModifiedNotice = document.getElementById("plan-modified-notice");
  addPolicyBtn = document.getElementById("add-policy-btn");
  addPolicyModal = document.getElementById("add-policy-modal");
  planAckModal = document.getElementById("plan-ack-modal");
}

// Attach planner event listeners
function attachPlannerEventListeners() {
  if (generatePlanBtn) {
    generatePlanBtn.addEventListener("click", generatePlan);
  }
  if (clearPlanBtn) {
    clearPlanBtn.addEventListener("click", clearPlan);
  }
  if (approvePlanBtn) {
    approvePlanBtn.addEventListener("click", handleApproveClick);
  }
  if (addPolicyBtn) {
    addPolicyBtn.addEventListener("click", openAddPolicyModal);
  }
}

// Load existing plan state
async function loadPlanState() {
  try {
    const plan = await invoke("cmd_get_latest_plan");
    if (plan) {
      currentPlan = plan;
      updatePlannerUI(plan);
    } else {
      resetPlannerUI();
    }
  } catch (error) {
    console.log("No existing plan:", error);
    resetPlannerUI();
  }
}

// Update planner UI with plan data
function updatePlannerUI(plan) {
  if (!plan) {
    resetPlannerUI();
    return;
  }
  
  currentPlan = plan;
  
  // Update badge
  if (plannerBadge) {
    if (plan.is_approved) {
      plannerBadge.textContent = "Approved";
      plannerBadge.className = "planner-badge approved";
    } else if (plan.is_user_modified) {
      plannerBadge.textContent = "Modified";
      plannerBadge.className = "planner-badge modified";
    } else {
      plannerBadge.textContent = "Plan Ready";
      plannerBadge.className = "planner-badge has-plan";
    }
  }
  
  // Show/hide modified notice
  if (planModifiedNotice) {
    planModifiedNotice.style.display = plan.is_user_modified ? "block" : "none";
  }
  
  // Update summary card
  if (planSummaryCard) {
    planSummaryCard.style.display = "block";
  }
  
  if (planGoalEl) {
    planGoalEl.textContent = plan.goal_description || "Achieve compliance threshold";
  }
  
  if (planSnapshotTimeEl) {
    planSnapshotTimeEl.textContent = plan.snapshot_timestamp || "N/A";
  }
  
  if (planComplianceRateEl) {
    const rate = plan.compliance_rate != null ? `${(plan.compliance_rate * 100).toFixed(1)}%` : "N/A";
    planComplianceRateEl.textContent = rate;
  }
  
  if (planGeneratedTimeEl) {
    const el = document.getElementById("plan-generated-at");
    if (el) el.textContent = plan.generated_at || "N/A";
  }
  
  if (planStepsCountEl) {
    planStepsCountEl.textContent = plan.steps ? plan.steps.length : 0;
  }
  
  if (planDeferredCountEl) {
    planDeferredCountEl.textContent = plan.deferred ? plan.deferred.length : 0;
  }
  
  // Update steps badge
  const stepsBadge = document.getElementById("steps-badge");
  if (stepsBadge) {
    stepsBadge.textContent = plan.steps ? plan.steps.length : 0;
  }
  
  // Update deferred badge
  const deferredBadge = document.getElementById("deferred-badge");
  if (deferredBadge) {
    deferredBadge.textContent = plan.deferred ? plan.deferred.length : 0;
  }
  
  // Update excluded badge
  const excludedBadge = document.getElementById("excluded-badge");
  if (excludedBadge) {
    excludedBadge.textContent = plan.excluded ? plan.excluded.length : 0;
  }
  
  // Update approval status
  if (approvalStatusEl) {
    if (plan.is_approved) {
      approvalStatusEl.innerHTML = '<span class="approval-icon">✅</span><span>Plan Approved</span>';
      approvalStatusEl.className = "approval-status approved";
    } else if (plan.is_user_modified) {
      approvalStatusEl.innerHTML = '<span class="approval-icon">✏️</span><span>Modified - Requires Re-approval</span>';
      approvalStatusEl.className = "approval-status";
    } else {
      approvalStatusEl.innerHTML = '<span class="approval-icon">🔒</span><span>Human Approval Required</span>';
      approvalStatusEl.className = "approval-status";
    }
  }
  
  // Update approve button
  if (approvePlanBtn) {
    approvePlanBtn.disabled = plan.is_approved;
    approvePlanBtn.textContent = plan.is_approved ? "✓ Approved" : "✅ Approve Plan";
  }
  
  // Enable clear button
  if (clearPlanBtn) {
    clearPlanBtn.disabled = false;
  }
  
  // Render steps with source indicators and remove buttons
  renderPlanSteps(plan.steps || []);
  
  // Render excluded policies
  renderExcludedPolicies(plan.excluded || []);
  
  // Render deferred
  renderDeferredPolicies(plan.deferred || []);
  
  // Show sections
  if (planStepsSection) {
    planStepsSection.style.display = "block";
  }
  if (planExcludedSection) {
    planExcludedSection.style.display = plan.excluded && plan.excluded.length > 0 ? "block" : "none";
  }
  if (planDeferredSection) {
    planDeferredSection.style.display = plan.deferred && plan.deferred.length > 0 ? "block" : "none";
  }
}

// Reset planner UI to initial state
function resetPlannerUI() {
  currentPlan = null;
  
  if (plannerBadge) {
    plannerBadge.textContent = "No Plan";
    plannerBadge.className = "planner-badge";
  }
  
  if (planSummaryCard) {
    planSummaryCard.style.display = "none";
  }
  
  if (planStepsSection) {
    planStepsSection.style.display = "none";
  }
  
  if (planDeferredSection) {
    planDeferredSection.style.display = "none";
  }
  
  if (planExcludedSection) {
    planExcludedSection.style.display = "none";
  }
  
  if (planModifiedNotice) {
    planModifiedNotice.style.display = "none";
  }
  
  if (clearPlanBtn) {
    clearPlanBtn.disabled = true;
  }
  
  if (approvePlanBtn) {
    approvePlanBtn.disabled = true;
  }
}

// Generate remediation plan
async function generatePlan() {
  if (generatePlanBtn) {
    generatePlanBtn.disabled = true;
    generatePlanBtn.textContent = "Generating...";
  }
  showLoading(true);
  
  try {
    const plan = await invoke("cmd_generate_remediation_plan");
    updatePlannerUI(plan);
    showNotification(
      `Plan generated: ${plan.steps.length} steps, ${plan.deferred.length} deferred`,
      "success"
    );
  } catch (error) {
    showNotification(`Failed to generate plan: ${error}`, "error");
  }
  
  showLoading(false);
  if (generatePlanBtn) {
    generatePlanBtn.disabled = false;
    generatePlanBtn.textContent = "🔄 Generate Plan";
  }
}

// Clear the current plan
async function clearPlan() {
  try {
    await invoke("cmd_clear_plan");
    resetPlannerUI();
    showNotification("Plan cleared", "success");
  } catch (error) {
    showNotification(`Failed to clear plan: ${error}`, "error");
  }
}

// Render plan steps with source indicators and remove buttons
function renderPlanSteps(steps) {
  if (!planStepsList) return;
  
  if (steps.length === 0) {
    planStepsList.innerHTML = '<p class="text-muted">No remediation steps in this plan.</p>';
    return;
  }
  
  planStepsList.innerHTML = steps.map((step, index) => {
    const riskClass = step.risk_score >= 7 ? 'risk-high' : (step.risk_score >= 4 ? 'risk-medium' : 'risk-low');
    const confClass = step.confidence >= 0.8 ? 'confidence-high' : (step.confidence >= 0.5 ? 'confidence-medium' : 'confidence-low');
    const isUserAdded = step.source === 'user';
    const sourceLabel = isUserAdded ? 'User Added' : 'System Proposed';
    const sourceClass = isUserAdded ? 'source-user' : 'source-planner';
    const itemClass = isUserAdded ? 'plan-step-item step-user-added' : 'plan-step-item';
    
    return `
      <div class="${itemClass}">
        <div class="step-priority">#${step.priority}</div>
        <div class="step-info" onclick="showStepDetails(${index})">
          <div style="display: flex; align-items: center; gap: 0.5rem;">
            <span class="step-policy-id">${step.policy_id}</span>
            <span class="step-source-badge ${sourceClass}">${sourceLabel}</span>
          </div>
          <span class="step-reason">${step.reason}</span>
        </div>
        <div class="step-metrics" onclick="showStepDetails(${index})">
          <div class="step-metric">
            <span class="metric-value ${riskClass}">${step.risk_score.toFixed(1)}</span>
            <span class="metric-label">Risk</span>
          </div>
          <div class="step-metric">
            <span class="metric-value ${confClass}">${(step.confidence * 100).toFixed(0)}%</span>
            <span class="metric-label">Conf</span>
          </div>
        </div>
        <div class="step-actions">
          <button class="btn-remove-step" onclick="event.stopPropagation(); removeStepFromPlan('${step.policy_id}')">
            Remove
          </button>
        </div>
      </div>
    `;
  }).join('');
}

// Render deferred policies
function renderDeferredPolicies(deferred) {
  if (!planDeferredList) return;
  
  if (deferred.length === 0) {
    planDeferredList.innerHTML = '<p class="text-muted">No deferred policies.</p>';
    return;
  }
  
  planDeferredList.innerHTML = deferred.map(item => {
    const constraintTags = item.blocking_constraints.map(c => 
      `<span class="constraint-tag">${c}</span>`
    ).join('');
    
    return `
      <div class="plan-deferred-item">
        <div class="deferred-info">
          <span class="deferred-policy-id">${item.policy_id}</span>
          <span class="deferred-reason">${item.reason}</span>
        </div>
        <div class="deferred-constraints">
          ${constraintTags || '<span class="constraint-tag">None specified</span>'}
        </div>
      </div>
    `;
  }).join('');
}

// Show step details modal
function showStepDetails(index) {
  if (!currentPlan || !currentPlan.steps || !currentPlan.steps[index]) {
    return;
  }
  
  const step = currentPlan.steps[index];
  
  // Update modal content
  document.getElementById("step-priority").textContent = `#${step.priority}`;
  document.getElementById("step-policy-id-detail").textContent = step.policy_id;
  
  const riskEl = document.getElementById("step-risk-score");
  riskEl.textContent = step.risk_score;
  riskEl.className = "detail-value risk-score";
  
  const confEl = document.getElementById("step-confidence");
  confEl.textContent = `${(step.confidence * 100).toFixed(0)}%`;
  confEl.className = "detail-value confidence";
  
  // Estimated duration and impact (from metadata if available)
  const metadata = currentPlan.metadata || {};
  document.getElementById("step-duration").textContent = metadata.estimated_duration || "~5 minutes";
  document.getElementById("step-impact").textContent = metadata.risk_assessment || "Low";
  
  document.getElementById("step-reason-text").textContent = step.reason;
  
  // Render constraints
  const constraintsEl = document.getElementById("step-constraints-list");
  if (step.constraints_considered && step.constraints_considered.length > 0) {
    constraintsEl.innerHTML = step.constraints_considered.map(c => 
      `<span class="constraint-tag">${c}</span>`
    ).join('');
  } else {
    constraintsEl.innerHTML = '<span class="no-constraints">No constraints specified</span>';
  }
  
  // Show modal
  if (plannerStepModal) {
    plannerStepModal.style.display = "block";
  }
}

// Close step details modal
function closeStepModal() {
  if (plannerStepModal) {
    plannerStepModal.style.display = "none";
  }
}

// Initialize planner
async function initializePlanner() {
  initializePlannerElements();
  attachPlannerEventListeners();
  await loadPlanState();
}

// ============================================================
// PLAN EDITING FUNCTIONS
// ============================================================

// Remove a step from the plan
async function removeStepFromPlan(policyId) {
  if (!currentPlan) {
    showNotification("No plan available", "warning");
    return;
  }
  
  try {
    const plan = await invoke("cmd_remove_plan_step", { policyId });
    updatePlannerUI(plan);
    showNotification(`Removed ${policyId} from plan`, "success");
  } catch (error) {
    showNotification(`Failed to remove step: ${error}`, "error");
  }
}

// Restore an excluded step back to the plan
async function restoreExcludedStep(policyId) {
  try {
    const plan = await invoke("cmd_restore_excluded_step", { policyId });
    updatePlannerUI(plan);
    showNotification(`Restored ${policyId} to plan`, "success");
  } catch (error) {
    showNotification(`Failed to restore step: ${error}`, "error");
  }
}

// Render excluded policies
function renderExcludedPolicies(excluded) {
  if (!planExcludedList) return;
  
  if (!excluded || excluded.length === 0) {
    planExcludedList.innerHTML = '<p class="text-muted">No manually excluded policies.</p>';
    return;
  }
  
  planExcludedList.innerHTML = excluded.map(item => {
    const sourceLabel = item.original_source === 'user' ? 'User Added' : 'System Proposed';
    return `
      <div class="plan-excluded-item">
        <div class="excluded-info">
          <span class="excluded-policy-id">${item.policy_id}</span>
          <span class="excluded-reason">${item.reason}</span>
          <span class="excluded-meta">Originally: ${sourceLabel} (Priority #${item.original_priority})</span>
        </div>
        <button class="btn-restore-step" onclick="restoreExcludedStep('${item.policy_id}')">
          Restore
        </button>
      </div>
    `;
  }).join('');
}

// Open the Add Policy modal
async function openAddPolicyModal() {
  if (!currentPlan) {
    showNotification("Generate a plan first before adding policies", "warning");
    return;
  }
  
  if (addPolicyModal) {
    addPolicyModal.style.display = "block";
  }
  
  // Load eligible policies
  await loadEligiblePolicies();
}

// Close the Add Policy modal
function closeAddPolicyModal() {
  if (addPolicyModal) {
    addPolicyModal.style.display = "none";
  }
  
  // Reset search
  const searchInput = document.getElementById("policy-search-input");
  if (searchInput) {
    searchInput.value = "";
  }
}

// Load eligible policies from backend
async function loadEligiblePolicies() {
  const listEl = document.getElementById("eligible-policies-list");
  if (!listEl) return;
  
  listEl.innerHTML = '<p class="loading-text">Loading policies...</p>';
  
  try {
    eligiblePolicies = await invoke("cmd_get_eligible_policies");
    filterEligiblePolicies();
  } catch (error) {
    listEl.innerHTML = `<p class="text-muted">Failed to load policies: ${error}</p>`;
  }
}

// Filter eligible policies based on search and filters
function filterEligiblePolicies() {
  const listEl = document.getElementById("eligible-policies-list");
  if (!listEl) return;
  
  const searchInput = document.getElementById("policy-search-input");
  const filterNonCompliant = document.getElementById("filter-non-compliant");
  
  const searchTerm = searchInput ? searchInput.value.toLowerCase() : "";
  const showOnlyNonCompliant = filterNonCompliant ? filterNonCompliant.checked : true;
  
  let filtered = eligiblePolicies.filter(p => {
    // Search filter
    const matchesSearch = !searchTerm || 
      p.policy_id.toLowerCase().includes(searchTerm) ||
      p.title.toLowerCase().includes(searchTerm) ||
      p.description.toLowerCase().includes(searchTerm);
    
    // Compliance filter
    const matchesCompliance = !showOnlyNonCompliant || !p.is_compliant;
    
    return matchesSearch && matchesCompliance;
  });
  
  if (filtered.length === 0) {
    listEl.innerHTML = '<p class="empty-policies-text">No matching policies found.</p>';
    return;
  }
  
  listEl.innerHTML = filtered.map(p => {
    const severityClass = p.severity.toLowerCase();
    const complianceClass = p.is_compliant ? 'compliant' : 'non-compliant';
    const complianceLabel = p.is_compliant ? '✓ Compliant' : '✗ Non-Compliant';
    const itemClasses = ['eligible-policy-item'];
    
    if (!p.can_add) itemClasses.push('policy-blocked');
    if (p.is_compliant) itemClasses.push('policy-compliant');
    
    return `
      <div class="${itemClasses.join(' ')}">
        <div class="policy-info">
          <div class="policy-info-header">
            <span class="policy-info-id">${p.policy_id}</span>
            <span class="policy-severity-badge ${severityClass}">${p.severity}</span>
            <span class="policy-compliance-badge ${complianceClass}">${complianceLabel}</span>
          </div>
          <span class="policy-info-title">${p.title}</span>
          ${p.block_reason ? `<span class="policy-block-reason">⚠️ ${p.block_reason}</span>` : ''}
        </div>
        <button class="btn-add-policy" 
                onclick="addPolicyToPlan('${p.policy_id}')"
                ${!p.can_add ? 'disabled' : ''}>
          ${p.can_add ? '+ Add' : 'Blocked'}
        </button>
      </div>
    `;
  }).join('');
}

// Add a policy to the plan
async function addPolicyToPlan(policyId) {
  try {
    const plan = await invoke("cmd_add_policy_to_plan", { policyId });
    updatePlannerUI(plan);
    closeAddPolicyModal();
    showNotification(`Added ${policyId} to plan`, "success");
  } catch (error) {
    showNotification(`Failed to add policy: ${error}`, "error");
  }
}

// Handle approve button click - show acknowledgment if modified
function handleApproveClick() {
  if (!currentPlan) {
    showNotification("No plan to approve", "warning");
    return;
  }
  
  if (currentPlan.is_user_modified) {
    // Show acknowledgment modal
    showAckModal();
  } else {
    // Direct approval
    approvePlan();
  }
}

// Show modification acknowledgment modal
function showAckModal() {
  if (!planAckModal || !currentPlan) return;
  
  // Build summary
  const summaryEl = document.getElementById("modification-summary");
  if (summaryEl) {
    const userAddedCount = currentPlan.steps.filter(s => s.source === 'user').length;
    const excludedCount = currentPlan.excluded ? currentPlan.excluded.length : 0;
    
    summaryEl.innerHTML = `
      <div class="modification-summary-item">
        <span>User-added steps:</span>
        <strong>${userAddedCount}</strong>
      </div>
      <div class="modification-summary-item">
        <span>Manually excluded:</span>
        <strong>${excludedCount}</strong>
      </div>
      <div class="modification-summary-item">
        <span>Total steps in plan:</span>
        <strong>${currentPlan.steps.length}</strong>
      </div>
    `;
  }
  
  planAckModal.style.display = "block";
}

// Close acknowledgment modal
function closeAckModal() {
  if (planAckModal) {
    planAckModal.style.display = "none";
  }
}

// Confirm approval after acknowledgment
async function confirmApproval() {
  closeAckModal();
  await approvePlan();
}

// Approve the current plan (UI acknowledgment only)
async function approvePlan() {
  if (!currentPlan) {
    showNotification("No plan to approve", "warning");
    return;
  }
  
  if (approvePlanBtn) {
    approvePlanBtn.disabled = true;
  }
  
  try {
    const plan = await invoke("cmd_approve_plan", { planId: currentPlan.plan_id });
    updatePlannerUI(plan);
    showNotification("Plan approved (no changes applied)", "success");
  } catch (error) {
    showNotification(`Failed to approve plan: ${error}`, "error");
    if (approvePlanBtn && currentPlan) {
      approvePlanBtn.disabled = currentPlan.is_approved;
    }
  }
}

// Make planner functions global for onclick handlers
window.generatePlan = generatePlan;
window.approvePlan = approvePlan;
window.clearPlan = clearPlan;
window.showStepDetails = showStepDetails;
window.closeStepModal = closeStepModal;
window.removeStepFromPlan = removeStepFromPlan;
window.restoreExcludedStep = restoreExcludedStep;
window.openAddPolicyModal = openAddPolicyModal;
window.closeAddPolicyModal = closeAddPolicyModal;
window.filterEligiblePolicies = filterEligiblePolicies;
window.addPolicyToPlan = addPolicyToPlan;
window.handleApproveClick = handleApproveClick;
window.showAckModal = showAckModal;
window.closeAckModal = closeAckModal;
window.confirmApproval = confirmApproval;
