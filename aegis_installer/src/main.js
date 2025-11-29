import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

// State
const state = {
    usbPath: '',
    keyPath: '',
    files: [],
    version: '1.0.0',
    isProcessing: false
};

// DOM Elements
const driveSelect = document.getElementById('driveSelect');
const refreshBtn = document.getElementById('refreshBtn');
const dropZone = document.getElementById('dropZone');
const fileInput = document.getElementById('fileInput');
const browseBtn = document.getElementById('browseBtn');
const fileList = document.getElementById('fileList');
const versionInput = document.getElementById('versionInput');
const keyPathDisplay = document.getElementById('keyPathDisplay');
const browseKeyBtn = document.getElementById('browseKeyBtn');
const prepareBtn = document.getElementById('prepareBtn');
const progressContainer = document.querySelector('.progress-container');
const progressBar = document.getElementById('progressBar');
const progressText = document.getElementById('progressText');
const statusLog = document.getElementById('statusLog');
const clearLogBtn = document.getElementById('clearLogBtn');
const successModal = document.getElementById('successModal');
const closeModalBtn = document.getElementById('closeModalBtn');
const repoPath = document.getElementById('repoPath');
const objectCount = document.getElementById('objectCount');

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    loadDrives();
    setupEventListeners();
});

// Event Listeners
function setupEventListeners() {
    refreshBtn.addEventListener('click', loadDrives);
    driveSelect.addEventListener('change', (e) => {
        state.usbPath = e.target.value;
        updatePrepareButton();
        log(`Selected drive: ${state.usbPath}`, 'info');
    });
    
    browseBtn.addEventListener('click', () => fileInput.click());
    fileInput.addEventListener('change', handleFileSelect);
    
    dropZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        dropZone.classList.add('drag-over');
    });
    
    dropZone.addEventListener('dragleave', () => {
        dropZone.classList.remove('drag-over');
    });
    
    dropZone.addEventListener('drop', handleFileDrop);
    
    versionInput.addEventListener('input', (e) => {
        state.version = e.target.value;
    });
    
    browseKeyBtn.addEventListener('click', selectKeyFile);
    
    prepareBtn.addEventListener('click', prepareUSB);
    
    clearLogBtn.addEventListener('click', () => {
        statusLog.innerHTML = '';
    });
    
    closeModalBtn.addEventListener('click', () => {
        successModal.classList.remove('active');
    });
}

// Load USB Drives
async function loadDrives() {
    try {
        log('Scanning for USB drives...', 'info');
        const drives = await invoke('cmd_list_drives');
        
        driveSelect.innerHTML = '<option value="">-- Select USB Drive --</option>';
        
        if (drives.length === 0) {
            log('No USB drives detected', 'warning');
            return;
        }
        
        drives.forEach(drive => {
            const option = document.createElement('option');
            option.value = drive;
            option.textContent = drive;
            driveSelect.appendChild(option);
        });
        
        log(`Found ${drives.length} drive(s)`, 'success');
    } catch (error) {
        log(`Failed to scan drives: ${error}`, 'error');
    }
}

// File Selection
function handleFileSelect(e) {
    const files = Array.from(e.target.files);
    addFiles(files);
}

function handleFileDrop(e) {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
    
    const files = Array.from(e.dataTransfer.files);
    addFiles(files);
}

async function addFiles(files) {
    for (const file of files) {
        // Check if already added
        if (state.files.some(f => f.name === file.name)) {
            log(`File already added: ${file.name}`, 'warning');
            continue;
        }
        
        log(`Computing hash for: ${file.name}...`, 'info');
        
        try {
            const arrayBuffer = await file.arrayBuffer();
            const hash = await computeSHA256(arrayBuffer);
            
            state.files.push({
                name: file.name,
                size: file.size,
                hash: hash,
                bytes: new Uint8Array(arrayBuffer)
            });
            
            log(`✓ ${file.name} - ${hash.substring(0, 16)}...`, 'success');
        } catch (error) {
            log(`Failed to process ${file.name}: ${error}`, 'error');
        }
    }
    
    renderFileList();
    updatePrepareButton();
}

function renderFileList() {
    if (state.files.length === 0) {
        fileList.innerHTML = '';
        return;
    }
    
    fileList.innerHTML = state.files.map((file, index) => `
        <div class="file-item">
            <div class="file-info">
                <div class="file-name">${file.name}</div>
                <div class="file-meta">${formatSize(file.size)}</div>
                <div class="file-hash">${file.hash}</div>
            </div>
            <button class="file-remove" data-index="${index}">Remove</button>
        </div>
    `).join('');
    
    // Attach remove handlers
    document.querySelectorAll('.file-remove').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const index = parseInt(e.target.dataset.index);
            state.files.splice(index, 1);
            renderFileList();
            updatePrepareButton();
        });
    });
}

// Select Key File
async function selectKeyFile() {
    try {
        const selected = await open({
            multiple: false,
            filters: [{
                name: 'Key Files',
                extensions: ['key', 'pem', 'priv']
            }]
        });
        
        if (selected) {
            state.keyPath = selected;
            keyPathDisplay.value = selected;
            log(`Private key selected: ${selected}`, 'success');
            updatePrepareButton();
        }
    } catch (error) {
        log(`Failed to select key: ${error}`, 'error');
    }
}

// Update Prepare Button State
function updatePrepareButton() {
    const canPrepare = state.usbPath && state.files.length > 0 && state.keyPath && !state.isProcessing;
    prepareBtn.disabled = !canPrepare;
}

// Prepare USB
async function prepareUSB() {
    if (state.isProcessing) return;
    
    state.isProcessing = true;
    prepareBtn.disabled = true;
    progressContainer.classList.add('active');
    
    try {
        // Step 1: Create repository structure
        log('Creating repository structure...', 'info');
        updateProgress(10, 'Creating directories...');
        await invoke('cmd_create_repo_structure', { usbPath: state.usbPath });
        log('✓ Repository structure created', 'success');
        
        // Step 2: Write objects
        log('Writing objects to USB...', 'info');
        const objectStep = 50 / state.files.length;
        
        for (let i = 0; i < state.files.length; i++) {
            const file = state.files[i];
            updateProgress(10 + (i + 1) * objectStep, `Writing ${file.name}...`);
            
            await invoke('cmd_write_object', {
                usbPath: state.usbPath,
                hash: file.hash,
                bytes: Array.from(file.bytes)
            });
            
            log(`✓ Written: ${file.name}`, 'success');
        }
        
        // Step 3: Generate manifest
        log('Generating manifest...', 'info');
        updateProgress(70, 'Generating manifest...');
        const manifest = generateManifest();
        
        // Step 4: Write manifest
        const manifestPath = `${state.usbPath}/aegis_repo/refs/heads/production.manifest`;
        await invoke('cmd_write_manifest', {
            usbPath: state.usbPath,
            manifest: manifest
        });
        log('✓ Manifest written', 'success');
        
        // Step 5: Sign manifest
        log('Signing manifest...', 'info');
        updateProgress(85, 'Signing manifest...');
        const signatureHex = await invoke('cmd_sign_manifest', {
            manifestPath: manifestPath,
            keyPath: state.keyPath
        });
        log(`✓ Signature: ${signatureHex.substring(0, 32)}...`, 'success');
        
        // Step 6: Write signature
        updateProgress(95, 'Writing signature...');
        await invoke('cmd_write_signature', {
            usbPath: state.usbPath,
            signatureHex: signatureHex
        });
        log('✓ Signature written', 'success');
        
        // Complete
        updateProgress(100, 'Complete!');
        log('🎉 USB drive prepared successfully!', 'success');
        
        // Show success modal
        repoPath.textContent = `${state.usbPath}/aegis_repo`;
        objectCount.textContent = state.files.length;
        successModal.classList.add('active');
        
    } catch (error) {
        log(`❌ Failed to prepare USB: ${error}`, 'error');
        updateProgress(0, 'Failed');
    } finally {
        state.isProcessing = false;
        setTimeout(() => {
            progressContainer.classList.remove('active');
            updatePrepareButton();
        }, 2000);
    }
}

// Generate Manifest
function generateManifest() {
    const manifest = {
        version: state.version,
        timestamp: new Date().toISOString(),
        file_count: state.files.length,
        objects: state.files.map(f => ({
            path: f.name,
            hash: f.hash,
            size: f.size
        })).sort((a, b) => a.hash.localeCompare(b.hash))
    };
    
    // Canonical JSON with sorted keys
    return JSON.stringify(manifest, Object.keys(manifest).sort(), 2);
}

// Compute SHA256 using Web Crypto
async function computeSHA256(arrayBuffer) {
    const hashBuffer = await crypto.subtle.digest('SHA-256', arrayBuffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

// Update Progress
function updateProgress(percent, text) {
    progressBar.style.setProperty('--progress', `${percent}%`);
    progressText.textContent = `${Math.round(percent)}%`;
    if (text) {
        log(text, 'info');
    }
}

// Logging
function log(message, type = 'info') {
    const timestamp = new Date().toLocaleTimeString();
    const entry = document.createElement('div');
    entry.className = `log-entry ${type}`;
    entry.textContent = `[${timestamp}] ${message}`;
    statusLog.appendChild(entry);
    statusLog.scrollTop = statusLog.scrollHeight;
}

// Format file size
function formatSize(bytes) {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}
