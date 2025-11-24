// State management
let state = {
    usbPath: '',
    files: [],
    version: '1.0.0',
    isProcessing: false
};

// DOM elements
const usbPathInput = document.getElementById('usbPath');
const browseBtn = document.getElementById('browseBtn');
const uploadZone = document.getElementById('uploadZone');
const fileInput = document.getElementById('fileInput');
const uploadBtn = document.getElementById('uploadBtn');
const fileList = document.getElementById('fileList');
const versionInput = document.getElementById('versionInput');
const prepareBtn = document.getElementById('prepareBtn');
const statusLog = document.getElementById('statusLog');
const successModal = document.getElementById('successModal');
const closeModal = document.getElementById('closeModal');
const repoPath = document.getElementById('repoPath');
const objectCount = document.getElementById('objectCount');

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    setupEventListeners();
    log('info', '🚀 USB Authority Creator initialized');
});

function setupEventListeners() {
    // USB path selection
    usbPathInput.addEventListener('input', (e) => {
        state.usbPath = e.target.value;
        updatePrepareButton();
    });

    browseBtn.addEventListener('click', browseForUSB);

    // File upload
    uploadBtn.addEventListener('click', () => fileInput.click());
    fileInput.addEventListener('change', handleFileSelect);

    // Drag and drop
    uploadZone.addEventListener('click', () => fileInput.click());
    uploadZone.addEventListener('dragover', handleDragOver);
    uploadZone.addEventListener('dragleave', handleDragLeave);
    uploadZone.addEventListener('drop', handleDrop);

    // Version input
    versionInput.addEventListener('input', (e) => {
        state.version = e.target.value;
    });

    // Prepare button
    prepareBtn.addEventListener('click', prepareUSB);

    // Modal
    closeModal.addEventListener('click', () => {
        successModal.classList.add('hidden');
    });
}

// File handling
function handleFileSelect(e) {
    const files = Array.from(e.target.files);
    processFiles(files);
}

function handleDragOver(e) {
    e.preventDefault();
    uploadZone.classList.add('drag-over');
}

function handleDragLeave(e) {
    e.preventDefault();
    uploadZone.classList.remove('drag-over');
}

function handleDrop(e) {
    e.preventDefault();
    uploadZone.classList.remove('drag-over');
    const files = Array.from(e.dataTransfer.files);
    processFiles(files);
}

async function processFiles(files) {
    log('info', `📦 Processing ${files.length} file(s)...`);

    for (const file of files) {
        const hash = await computeSHA256(file);
        const fileObj = {
            name: file.name,
            size: file.size,
            hash: hash,
            file: file
        };
        
        state.files.push(fileObj);
        addFileToList(fileObj);
    }

    updatePrepareButton();
    log('success', `✅ Added ${files.length} file(s) to queue`);
}

function addFileToList(fileObj) {
    const fileItem = document.createElement('div');
    fileItem.className = 'file-item';
    fileItem.dataset.hash = fileObj.hash;

    fileItem.innerHTML = `
        <div class="file-info">
            <div class="file-icon">📄</div>
            <div class="file-details">
                <div class="file-name">${fileObj.name}</div>
                <div class="file-size">${formatBytes(fileObj.size)}</div>
                <div class="file-hash">${fileObj.hash.substring(0, 16)}...</div>
            </div>
        </div>
        <button class="remove-btn" onclick="removeFile('${fileObj.hash}')">✕</button>
    `;

    fileList.appendChild(fileItem);
}

function removeFile(hash) {
    state.files = state.files.filter(f => f.hash !== hash);
    const fileItem = document.querySelector(`[data-hash="${hash}"]`);
    if (fileItem) {
        fileItem.remove();
    }
    updatePrepareButton();
    log('info', '🗑️ Removed file from queue');
}

// SHA256 computation
async function computeSHA256(file) {
    const buffer = await file.arrayBuffer();
    const hashBuffer = await crypto.subtle.digest('SHA-256', buffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
    return hashHex;
}

// USB browsing (Tauri invoke)
async function browseForUSB() {
    try {
        if (window.__TAURI__) {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                multiple: false,
                title: 'Select USB Drive'
            });

            if (selected) {
                usbPathInput.value = selected;
                state.usbPath = selected;
                updatePrepareButton();
                log('success', `✅ Selected USB: ${selected}`);
            }
        } else {
            log('error', '⚠️ Tauri not available. Enter path manually.');
        }
    } catch (error) {
        log('error', `❌ Failed to browse: ${error.message}`);
    }
}

// Prepare USB repository
async function prepareUSB() {
    if (state.isProcessing) return;
    
    state.isProcessing = true;
    prepareBtn.disabled = true;
    statusLog.classList.add('active');
    statusLog.innerHTML = '';

    try {
        log('info', '🚀 Starting AegisPack preparation...');
        log('info', `📍 Target: ${state.usbPath}/aegis_repo`);

        // Step 1: Create directory structure
        log('info', '📁 Creating directory structure...');
        await createDirectoryStructure();
        log('success', '✅ Directories created');

        // Step 2: Copy objects to CAS
        log('info', '📦 Copying objects to content-addressable storage...');
        await copyObjectsToCAS();
        log('success', `✅ Copied ${state.files.length} object(s)`);

        // Step 3: Generate canonical manifest
        log('info', '📝 Generating canonical manifest...');
        const manifest = generateCanonicalManifest();
        log('success', '✅ Manifest generated');

        // Step 4: Write manifest
        log('info', '💾 Writing manifest to USB...');
        await writeManifest(manifest);
        log('success', '✅ Manifest written');

        // Step 5: Sign manifest
        log('info', '🔐 Signing manifest with Ed25519...');
        await signManifest();
        log('success', '✅ Manifest signed');

        // Success
        log('success', '🎉 AegisPack repository created successfully!');
        showSuccessModal();

    } catch (error) {
        log('error', `❌ Error: ${error.message}`);
        alert(`Failed to prepare USB: ${error.message}`);
    } finally {
        state.isProcessing = false;
        prepareBtn.disabled = false;
    }
}

async function createDirectoryStructure() {
    const repoPath = `${state.usbPath}/aegis_repo`;
    
    if (window.__TAURI__) {
        await window.__TAURI__.core.invoke('cmd_create_directory', {
            path: `${repoPath}/objects`
        });
        await window.__TAURI__.core.invoke('cmd_create_directory', {
            path: `${repoPath}/refs/heads`
        });
    } else {
        throw new Error('Tauri API not available');
    }
}

async function copyObjectsToCAS() {
    const repoPath = `${state.usbPath}/aegis_repo`;

    for (const fileObj of state.files) {
        const hash = fileObj.hash;
        const shardDir = hash.substring(0, 2);
        const remaining = hash.substring(2);
        const objectDir = `${repoPath}/objects/${shardDir}`;
        const objectPath = `${objectDir}/${remaining}`;

        // Create shard directory
        if (window.__TAURI__) {
            await window.__TAURI__.core.invoke('cmd_create_directory', {
                path: objectDir
            });

            // Copy file
            const fileContent = await fileObj.file.arrayBuffer();
            await window.__TAURI__.core.invoke('cmd_write_file', {
                path: objectPath,
                content: Array.from(new Uint8Array(fileContent))
            });

            log('info', `  ├─ ${fileObj.name} → ${shardDir}/${remaining.substring(0, 12)}...`);
        }
    }
}

function generateCanonicalManifest() {
    // Create manifest with sorted objects
    const objects = state.files.map(f => ({
        hash: f.hash,
        size: f.size
    })).sort((a, b) => a.hash.localeCompare(b.hash));

    const manifest = {
        objects: objects,
        version: state.version
    };

    // Canonical JSON: sorted keys, no extra whitespace
    return JSON.stringify(manifest, Object.keys(manifest).sort(), 2);
}

async function writeManifest(manifestContent) {
    const manifestPath = `${state.usbPath}/aegis_repo/refs/heads/production.manifest`;

    if (window.__TAURI__) {
        await window.__TAURI__.core.invoke('cmd_write_text_file', {
            path: manifestPath,
            content: manifestContent
        });
    } else {
        throw new Error('Tauri API not available');
    }
}

async function signManifest() {
    const manifestPath = `${state.usbPath}/aegis_repo/refs/heads/production.manifest`;
    const sigPath = `${state.usbPath}/aegis_repo/refs/heads/production.sig`;

    if (window.__TAURI__) {
        await window.__TAURI__.core.invoke('cmd_sign_manifest', {
            manifestPath: manifestPath,
            sigPath: sigPath
        });
    } else {
        throw new Error('Tauri API not available');
    }
}

function showSuccessModal() {
    repoPath.textContent = `${state.usbPath}/aegis_repo`;
    objectCount.textContent = state.files.length;
    successModal.classList.remove('hidden');
}

// Utility functions
function updatePrepareButton() {
    prepareBtn.disabled = !state.usbPath || state.files.length === 0 || state.isProcessing;
}

function log(type, message) {
    const entry = document.createElement('div');
    entry.className = `log-entry ${type}`;
    entry.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
    statusLog.appendChild(entry);
    statusLog.scrollTop = statusLog.scrollHeight;
}

function formatBytes(bytes) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
}
