// CSV Import Page Logic
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

// State
let csvData = [];
let filteredData = [];
let currentPage = 1;
const rowsPerPage = 50;

// Elements
let csvFileInput;
let selectFileBtn;
let importUsbCsvBtn;
let backToDashboardBtn;
let fileNameDisplay;
let validationStatus;
let loading;
let reportMetadata;
let summarySection;
let csvTableContainer;
let csvTableBody;
let tableSearch;
let statusFilterCsv;
let severityFilterCsv;
let prevPageBtn;
let nextPageBtn;
let pageInfo;
let emptyState;

// Initialize
document.addEventListener("DOMContentLoaded", () => {
  initializeElements();
  attachEventListeners();
});

function initializeElements() {
  csvFileInput = document.getElementById("csv-file-input");
  selectFileBtn = document.getElementById("select-file-btn");
  importUsbCsvBtn = document.getElementById("import-usb-csv-btn");
  backToDashboardBtn = document.getElementById("back-to-dashboard-btn");
  fileNameDisplay = document.getElementById("file-name-display");
  validationStatus = document.getElementById("validation-status");
  loading = document.getElementById("loading");
  reportMetadata = document.getElementById("report-metadata");
  summarySection = document.getElementById("summary-section");
  csvTableContainer = document.getElementById("csv-table-container");
  csvTableBody = document.getElementById("csv-table-body");
  tableSearch = document.getElementById("table-search");
  statusFilterCsv = document.getElementById("status-filter-csv");
  severityFilterCsv = document.getElementById("severity-filter-csv");
  prevPageBtn = document.getElementById("prev-page-btn");
  nextPageBtn = document.getElementById("next-page-btn");
  pageInfo = document.getElementById("page-info");
  emptyState = document.getElementById("empty-state");
}

function attachEventListeners() {
  selectFileBtn.addEventListener("click", () => csvFileInput.click());
  csvFileInput.addEventListener("change", handleFileSelect);
  importUsbCsvBtn.addEventListener("click", handleUsbImport);
  backToDashboardBtn.addEventListener("click", () => {
    window.location.href = "index.html";
  });
  
  tableSearch.addEventListener("input", handleFilters);
  statusFilterCsv.addEventListener("change", handleFilters);
  severityFilterCsv.addEventListener("change", handleFilters);
  
  prevPageBtn.addEventListener("click", () => changePage(-1));
  nextPageBtn.addEventListener("click", () => changePage(1));
}

async function handleFileSelect(event) {
  const file = event.target.files[0];
  if (!file) return;

  fileNameDisplay.textContent = file.name;
  showLoading(true);
  hideValidationStatus();

  try {
    const text = await file.text();
    await parseAndValidateCsv(text, file.name);
  } catch (error) {
    showValidationError(`Failed to read file: ${error.message}`);
    console.error("File read error:", error);
  } finally {
    showLoading(false);
  }
}

async function handleUsbImport() {
  showLoading(true);
  hideValidationStatus();

  try {
    // Try to detect USB mount and find CSV
    const usbPath = await invoke("detect_usb_csv_reports");
    
    if (!usbPath || usbPath.length === 0) {
      showValidationError("No USB-B device detected or no reports found. Please ensure USB-B is connected with reports directory.");
      showLoading(false);
      return;
    }

    // If multiple hosts found, let user select
    let selectedPath;
    if (Array.isArray(usbPath) && usbPath.length > 1) {
      selectedPath = await selectHostReport(usbPath);
      if (!selectedPath) {
        showLoading(false);
        return;
      }
    } else {
      selectedPath = Array.isArray(usbPath) ? usbPath[0] : usbPath;
    }

    // Read the CSV file from USB
    const csvContent = await invoke("read_csv_file", { path: selectedPath });
    const fileName = selectedPath.split('/').pop();
    
    fileNameDisplay.textContent = `USB: ${fileName}`;
    await parseAndValidateCsv(csvContent, fileName);
  } catch (error) {
    showValidationError(`USB import failed: ${error.message}`);
    console.error("USB import error:", error);
  } finally {
    showLoading(false);
  }
}

async function selectHostReport(paths) {
  // Create a simple modal to select from multiple host reports
  const hostnames = paths.map(p => {
    const parts = p.split('/');
    return parts[parts.length - 2]; // Get hostname from path
  });

  const selected = await showSelectionModal("Select Host Report", hostnames);
  if (selected !== null) {
    return paths[selected];
  }
  return null;
}

function showSelectionModal(title, options) {
  return new Promise((resolve) => {
    const modal = document.createElement('div');
    modal.className = 'modal-overlay';
    modal.innerHTML = `
      <div class="modal-content" style="max-width: 400px;">
        <h3>${title}</h3>
        <div style="margin: 1rem 0;">
          ${options.map((opt, idx) => `
            <button class="btn btn-secondary" style="width: 100%; margin-bottom: 0.5rem;" data-index="${idx}">
              ${opt}
            </button>
          `).join('')}
        </div>
        <button class="btn btn-secondary cancel-btn">Cancel</button>
      </div>
    `;
    
    document.body.appendChild(modal);
    
    modal.querySelectorAll('.btn:not(.cancel-btn)').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const index = parseInt(e.target.dataset.index);
        document.body.removeChild(modal);
        resolve(index);
      });
    });
    
    modal.querySelector('.cancel-btn').addEventListener('click', () => {
      document.body.removeChild(modal);
      resolve(null);
    });
  });
}

async function parseAndValidateCsv(csvText, fileName) {
  return new Promise((resolve, reject) => {
    Papa.parse(csvText, {
      header: true,
      skipEmptyLines: true,
      complete: (results) => {
        try {
          validateCsvStructure(results);
          csvData = results.data;
          filteredData = [...csvData];
          
          displayMetadata(fileName);
          displaySummary();
          displayTable();
          
          showValidationSuccess(`Successfully imported ${csvData.length} records`);
          showContent();
          resolve();
        } catch (error) {
          showValidationError(`CSV validation failed: ${error.message}`);
          reject(error);
        }
      },
      error: (error) => {
        showValidationError(`CSV parsing failed: ${error.message}`);
        reject(error);
      }
    });
  });
}

function validateCsvStructure(results) {
  const requiredColumns = [
    "policy_id",
    "description",
    "expected",
    "actual",
    "status",
    "severity",
    "timestamp"
  ];

  const headers = results.meta.fields;
  
  // Check all required columns exist
  const missingColumns = requiredColumns.filter(col => !headers.includes(col));
  if (missingColumns.length > 0) {
    throw new Error(`Missing required columns: ${missingColumns.join(", ")}`);
  }

  // Validate data rows
  if (results.data.length === 0) {
    throw new Error("CSV file is empty");
  }

  // Validate severity values
  const validSeverities = ["high", "medium", "low"];
  const invalidRows = results.data.filter(row => 
    row.severity && !validSeverities.includes(row.severity.toLowerCase())
  );
  
  if (invalidRows.length > 0) {
    console.warn(`Warning: ${invalidRows.length} rows have invalid severity values`);
  }

  // Validate timestamps (RFC3339 format)
  const invalidTimestamps = results.data.filter(row => {
    if (!row.timestamp) return true;
    const date = new Date(row.timestamp);
    return isNaN(date.getTime());
  });

  if (invalidTimestamps.length > 0) {
    console.warn(`Warning: ${invalidTimestamps.length} rows have invalid timestamps`);
  }

  console.log("CSV validation successful");
}

function displayMetadata(fileName) {
  // Extract hostname from filename or path
  let hostname = "Unknown";
  const pathParts = fileName.split('/');
  if (pathParts.length > 2) {
    hostname = pathParts[pathParts.length - 2];
  } else if (fileName.includes('_')) {
    hostname = fileName.split('_')[0];
  }

  // Get latest timestamp
  let latestTimestamp = "Unknown";
  if (csvData.length > 0 && csvData[0].timestamp) {
    const dates = csvData
      .map(row => new Date(row.timestamp))
      .filter(date => !isNaN(date.getTime()));
    
    if (dates.length > 0) {
      const latest = new Date(Math.max(...dates));
      latestTimestamp = latest.toLocaleString();
    }
  }

  // Calculate statistics
  const total = csvData.length;
  const passed = csvData.filter(row => row.status?.toUpperCase() === "PASS").length;
  const failed = total - passed;
  const score = total > 0 ? ((passed / total) * 100).toFixed(1) : "0.0";

  document.getElementById("meta-hostname").textContent = hostname;
  document.getElementById("meta-timestamp").textContent = latestTimestamp;
  document.getElementById("meta-total").textContent = total;
  document.getElementById("meta-passed").textContent = passed;
  document.getElementById("meta-failed").textContent = failed;
  document.getElementById("meta-score").textContent = `${score}%`;
}

function displaySummary() {
  const highCount = csvData.filter(row => 
    row.severity?.toLowerCase() === "high"
  ).length;
  
  const mediumCount = csvData.filter(row => 
    row.severity?.toLowerCase() === "medium"
  ).length;
  
  const lowCount = csvData.filter(row => 
    row.severity?.toLowerCase() === "low"
  ).length;

  document.getElementById("severity-high-count").textContent = highCount;
  document.getElementById("severity-medium-count").textContent = mediumCount;
  document.getElementById("severity-low-count").textContent = lowCount;
}

function displayTable() {
  currentPage = 1;
  renderTablePage();
  updatePaginationControls();
}

function renderTablePage() {
  const start = (currentPage - 1) * rowsPerPage;
  const end = start + rowsPerPage;
  const pageData = filteredData.slice(start, end);

  csvTableBody.innerHTML = pageData.map(row => `
    <tr>
      <td>${escapeHtml(row.policy_id || "-")}</td>
      <td>${escapeHtml(row.description || "-")}</td>
      <td>${escapeHtml(row.expected || "-")}</td>
      <td>${escapeHtml(row.actual || "-")}</td>
      <td class="${row.status?.toUpperCase() === "PASS" ? "status-pass" : "status-fail"}">
        ${escapeHtml(row.status || "-")}
      </td>
      <td>
        <span class="severity-badge severity-${row.severity?.toLowerCase()}">
          ${escapeHtml(row.severity || "-")}
        </span>
      </td>
      <td>${formatTimestamp(row.timestamp)}</td>
    </tr>
  `).join('');
}

function updatePaginationControls() {
  const totalPages = Math.ceil(filteredData.length / rowsPerPage);
  
  prevPageBtn.disabled = currentPage === 1;
  nextPageBtn.disabled = currentPage >= totalPages || totalPages === 0;
  pageInfo.textContent = `Page ${currentPage} of ${totalPages}`;
}

function changePage(delta) {
  const totalPages = Math.ceil(filteredData.length / rowsPerPage);
  const newPage = currentPage + delta;
  
  if (newPage >= 1 && newPage <= totalPages) {
    currentPage = newPage;
    renderTablePage();
    updatePaginationControls();
  }
}

function handleFilters() {
  const searchTerm = tableSearch.value.toLowerCase();
  const statusFilter = statusFilterCsv.value;
  const severityFilter = severityFilterCsv.value;

  filteredData = csvData.filter(row => {
    // Search filter
    const matchesSearch = !searchTerm || 
      row.policy_id?.toLowerCase().includes(searchTerm) ||
      row.description?.toLowerCase().includes(searchTerm);

    // Status filter
    const matchesStatus = statusFilter === "all" || 
      row.status?.toUpperCase() === statusFilter;

    // Severity filter
    const matchesSeverity = severityFilter === "all" || 
      row.severity?.toLowerCase() === severityFilter;

    return matchesSearch && matchesStatus && matchesSeverity;
  });

  currentPage = 1;
  renderTablePage();
  updatePaginationControls();
}

function showContent() {
  emptyState.style.display = "none";
  reportMetadata.style.display = "block";
  summarySection.style.display = "block";
  csvTableContainer.style.display = "block";
}

function showLoading(show) {
  loading.style.display = show ? "flex" : "none";
}

function showValidationSuccess(message) {
  validationStatus.className = "validation-status success";
  validationStatus.textContent = `✓ ${message}`;
  validationStatus.style.display = "block";
  
  // Log success
  console.log(`CSV Import Success: ${message}`);
}

function showValidationError(message) {
  validationStatus.className = "validation-status error";
  validationStatus.textContent = `✗ ${message}`;
  validationStatus.style.display = "block";
  
  // Log error
  console.error(`CSV Import Error: ${message}`);
}

function hideValidationStatus() {
  validationStatus.style.display = "none";
}

function formatTimestamp(timestamp) {
  if (!timestamp) return "-";
  
  try {
    const date = new Date(timestamp);
    if (isNaN(date.getTime())) return timestamp;
    
    return date.toLocaleString();
  } catch {
    return timestamp;
  }
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Add severity badge styles
const style = document.createElement('style');
style.textContent = `
  .severity-badge {
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.875rem;
    font-weight: bold;
    text-transform: uppercase;
  }
  
  .severity-badge.severity-high {
    background: #ffebee;
    color: #c62828;
  }
  
  .severity-badge.severity-medium {
    background: #fff3e0;
    color: #ef6c00;
  }
  
  .severity-badge.severity-low {
    background: #e8f5e9;
    color: #2e7d32;
  }
  
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  
  .modal-content {
    background: white;
    padding: 2rem;
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }
`;
document.head.appendChild(style);
