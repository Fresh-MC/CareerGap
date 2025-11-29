const { invoke } = window.__TAURI__.core;

// State management
let policies = [];
let auditResults = {};
let filteredPolicies = [];

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
    filteredPolicies = [...policies];
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

  renderPolicies();
}

function renderPolicies() {
  if (filteredPolicies.length === 0) {
    policiesContainer.innerHTML = '<div class="empty-state"><p>No policies found</p></div>';
    return;
  }

  policiesContainer.innerHTML = filteredPolicies.map(policy => {
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
  }).join('');
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

// Make functions global for onclick handlers
window.auditPolicy = auditPolicy;
window.remediatePolicy = remediatePolicy;
window.showPolicyDetails = showPolicyDetails;
window.rollbackPolicy = rollbackPolicy;
window.rollbackAll = rollbackAll;
window.generateReport = generateReport;
window.exportCsvReport = exportCsvReport;
window.exportReportToPdf = exportReportToPdf;
