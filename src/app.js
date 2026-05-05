// ========== STATE ==========
let currentLang = 'ru';
let extractedFiles = [];
let currentPreviewId = null;

const __TAURI__ = window.__TAURI__;

// ========== Tauri invoke helper ==========
async function invoke(cmd, args) {
    try {
        if (__TAURI__ && __TAURI__.core) {
            return await __TAURI__.core.invoke(cmd, args || {});
        }
        // Fallback for older Tauri versions
        if (window.__TAURI_INTERNALS__) {
            const result = await window.__TAURI_INTERNALS__.invoke(cmd, args || {});
            return result;
        }
        throw new Error('Tauri API not available');
    } catch (err) {
        console.error(`Error invoking ${cmd}:`, err);
        showStatus(err.toString(), 'error');
        throw err;
    }
}

// ========== LOCALIZATION ==========
const i18nData = {};

async function loadI18n() {
    // i18n is handled by the Rust backend; we just toggle class on lang buttons
}

function setLang(lang) {
    currentLang = lang;
    document.querySelectorAll('.lang-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.lang === lang);
    });
    updateAllTexts();
}

function t(key) {
    // Simple client-side fallback translations
    const ru = {
        'app_name': 'DeepSeek Extractor',
        'paste_from_clipboard': '📋 Вставить из буфера',
        'extract': '🔍 Извлечь',
        'extract_from_url': '🔍 Извлечь',
        'save_to_folder': '💾 Сохранить в папку',
        'create_archive': '📦 Создать архив',
        'extract_archive': '📂 Извлечь архив',
        'files_found': 'Найдено файлов',
        'total_size': 'Общий размер',
        'preview': 'Предпросмотр',
        'file_name': 'Имя файла',
        'path': 'Путь',
        'language': 'Язык',
        'size': 'Размер',
        'actions': 'Действия',
        'select_all': 'Выбрать все',
        'url_placeholder': 'Вставьте URL с диалогом ИИ...',
        'archive_path': 'Путь к архиву',
        'status_ready': 'Готов к работе',
        'status_extracting': 'Извлечение файлов...',
        'status_saving': 'Сохранение файлов...',
        'status_archiving': 'Создание архива...',
        'status_extracting_archive': 'Извлечение архива...',
        'drag_drop_hint': 'Перетащите текстовый файл сюда',
        'reset_learning': '🔄 Сбросить обучение',
        'lang_ru': 'RU',
        'lang_en': 'EN',
        'confirm_overwrite': 'Некоторые файлы уже существуют. Перезаписать?',
        'learning_model_updated': 'Модель обучения обновлена',
        'error_no_files': 'Нет извлечённых файлов',
        'rename': '✏️',
        'delete': '❌',
        'version': 'Версия 1.0.0',
        'save': 'Сохранить',
        'cancel': 'Отмена',
        'rename_title': 'Переименовать файл',
        'enter_new_name': 'Введите новое имя файла',
        'overwrite_confirm': 'Подтверждение перезаписи',
        'confirm': 'Подтвердить',
        'no_files_in_archive': 'В архиве нет файлов',
        'archive_created': 'Архив создан',
        'archives': 'Архивы CodePack (*.cpk)',
        'all_files': 'Все файлы',
    };
    const en = {
        'app_name': 'DeepSeek Extractor',
        'paste_from_clipboard': '📋 Paste from Clipboard',
        'extract': '🔍 Extract',
        'extract_from_url': '🔍 Extract',
        'save_to_folder': '💾 Save to Folder',
        'create_archive': '📦 Create Archive',
        'extract_archive': '📂 Extract Archive',
        'files_found': 'Files found',
        'total_size': 'Total size',
        'preview': 'Preview',
        'file_name': 'File name',
        'path': 'Path',
        'language': 'Language',
        'size': 'Size',
        'actions': 'Actions',
        'select_all': 'Select All',
        'url_placeholder': 'Paste AI conversation URL...',
        'archive_path': 'Archive path',
        'status_ready': 'Ready',
        'status_extracting': 'Extracting files...',
        'status_saving': 'Saving files...',
        'status_archiving': 'Creating archive...',
        'status_extracting_archive': 'Extracting archive...',
        'drag_drop_hint': 'Drop a text file here',
        'reset_learning': '🔄 Reset Learning',
        'lang_ru': 'RU',
        'lang_en': 'EN',
        'confirm_overwrite': 'Some files already exist. Overwrite?',
        'learning_model_updated': 'Learning model updated',
        'error_no_files': 'No extracted files',
        'rename': '✏️',
        'delete': '❌',
        'version': 'Version 1.0.0',
        'save': 'Save',
        'cancel': 'Cancel',
        'rename_title': 'Rename File',
        'enter_new_name': 'Enter new file name',
        'overwrite_confirm': 'Overwrite Confirmation',
        'confirm': 'Confirm',
        'no_files_in_archive': 'No files in archive',
        'archive_created': 'Archive created',
        'archives': 'CodePack Archives (*.cpk)',
        'all_files': 'All Files',
    };
    const dict = currentLang === 'ru' ? ru : en;
    return dict[key] || key;
}

function updateAllTexts() {
    // Update data-i18n elements
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.dataset.i18n;
        const text = t(key);
        if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {
            el.placeholder = t(el.dataset.i18nPlaceholder || key);
        } else {
            // Don't override innerHTML of elements with children (like buttons with nested content)
            if (!el.querySelector('*')) {
                el.textContent = text;
            }
        }
    });

    // Update placeholders
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
        el.placeholder = t(el.dataset.i18nPlaceholder);
    });

    // Update status
    updateStatusBar();
}

// ========== EXTRACTOR COMMANDS ==========
async function pasteFromClipboard() {
    try {
        showStatus(t('status_extracting'), 'info');
        const result = await invoke('extract_from_clipboard');
        handleExtractResult(result);
    } catch (err) {
        showStatus('Error: ' + err.toString(), 'error');
    }
}

async function fetchFromUrl() {
    const url = document.getElementById('url-input').value.trim();
    if (!url) {
        showStatus(currentLang === 'ru' ? 'Введите URL' : 'Enter URL', 'warning');
        return;
    }
    try {
        showStatus(t('status_extracting'), 'info');
        const result = await invoke('extract_from_url', { url: url });
        handleExtractResult(result);
    } catch (err) {
        showStatus('Error: ' + err.toString(), 'error');
    }
}

function handleExtractResult(result) {
    extractedFiles = result.files || [];
    renderTable();
    updateStatusBar();

    document.getElementById('main-content').style.display = 'flex';

    if (extractedFiles.length === 0) {
        showStatus(t('error_no_files'), 'warning');
    } else {
        showStatus(`${t('files_found')}: ${extractedFiles.length}`, 'success');
    }
}

// ========== SAVE / ARCHIVE COMMANDS ==========
async function saveToFolder() {
    const selected = extractedFiles.filter(f => f.selected);
    if (selected.length === 0) {
        showStatus(t('error_no_files'), 'warning');
        return;
    }

    // Use Tauri dialog to pick folder
    let folderPath = null;
    try {
        if (__TAURI__ && __TAURI__.dialog) {
            folderPath = await __TAURI__.dialog.open({ directory: true });
        }
    } catch (e) {
        // Dialog not available, prompt manually
        folderPath = prompt(currentLang === 'ru' ? 'Путь к папке:' : 'Output folder path:');
    }

    if (!folderPath) return;

    try {
        showStatus(t('status_saving'), 'info');
        const count = await invoke('save_files', {
            files: selected,
            basePath: folderPath
        });
        showStatus(`${currentLang === 'ru' ? 'Сохранено' : 'Saved'} ${count} ${currentLang === 'ru' ? 'файлов' : 'files'}`, 'success');
    } catch (err) {
        showStatus('Error: ' + err.toString(), 'error');
    }
}

async function createArchive() {
    const selected = extractedFiles.filter(f => f.selected);
    if (selected.length === 0) {
        showStatus(t('error_no_files'), 'warning');
        return;
    }

    let savePath = null;
    try {
        if (__TAURI__ && __TAURI__.dialog) {
            savePath = await __TAURI__.dialog.save({
                filters: [{ name: t('archives'), extensions: ['cpk'] }]
            });
        }
    } catch (e) {
        const archiveInput = document.getElementById('archive-path');
        savePath = archiveInput.value.trim() || prompt(currentLang === 'ru' ? 'Путь для архива:' : 'Archive save path:');
    }

    if (!savePath) return;

    try {
        showStatus(t('status_archiving'), 'info');
        const info = await invoke('create_archive', {
            files: selected,
            savePath: savePath
        });
        showStatus(`${t('archive_created')}: ${info.file_count} ${currentLang === 'ru' ? 'файлов' : 'files'}`, 'success');
    } catch (err) {
        showStatus('Error: ' + err.toString(), 'error');
    }
}

async function extractArchive() {
    let archivePath = document.getElementById('archive-path').value.trim();
    if (!archivePath) {
        try {
            if (__TAURI__ && __TAURI__.dialog) {
                archivePath = await __TAURI__.dialog.open({
                    filters: [{ name: t('archives'), extensions: ['cpk'] }]
                });
            }
        } catch (e) {
            archivePath = prompt(currentLang === 'ru' ? 'Путь к архиву:' : 'Archive path:');
        }
    }

    if (!archivePath) return;

    let outputDir = null;
    try {
        if (__TAURI__ && __TAURI__.dialog) {
            outputDir = await __TAURI__.dialog.open({ directory: true });
        }
    } catch (e) {
        outputDir = prompt(currentLang === 'ru' ? 'Папка для извлечения:' : 'Output directory:');
    }

    if (!outputDir) return;

    try {
        showStatus(t('status_extracting_archive'), 'info');
        const files = await invoke('extract_archive', {
            archivePath: archivePath,
            outputDir: outputDir
        });
        extractedFiles = files || [];
        renderTable();
        document.getElementById('main-content').style.display = 'flex';
        showStatus(`${t('files_found')}: ${extractedFiles.length}`, 'success');
    } catch (err) {
        showStatus('Error: ' + err.toString(), 'error');
    }
}

// ========== PREVIEW ==========
async function previewFile(fileId) {
    const file = extractedFiles.find(f => f.id === fileId);
    if (!file) return;

    currentPreviewId = fileId;
    const previewContent = document.getElementById('preview-content');

    try {
        const html = await invoke('preview_file', { file: file });
        previewContent.innerHTML = html;
    } catch (err) {
        // Fallback: show plain text with escaping
        const escaped = file.content
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
        previewContent.innerHTML = `<pre style="background:#0a0e1a;color:#d4d4d4;padding:16px;border-radius:8px;overflow:auto;font-family:monospace;font-size:13px;line-height:1.5;"><code>${escaped}</code></pre>`;
    }

    // Highlight selected row
    document.querySelectorAll('#file-table-body tr').forEach(tr => {
        tr.classList.toggle('selected', tr.dataset.id === fileId);
    });
}

// ========== TABLE RENDERING ==========
function renderTable() {
    const tbody = document.getElementById('file-table-body');
    tbody.innerHTML = '';

    if (extractedFiles.length === 0) {
        document.getElementById('main-content').style.display = 'none';
        return;
    }

    extractedFiles.forEach((file, index) => {
        const tr = document.createElement('tr');
        tr.dataset.id = file.id;
        tr.dataset.index = index;
        tr.style.animationDelay = `${index * 0.05}s`;

        tr.innerHTML = `
            <td class="col-checkbox">
                <input type="checkbox" ${file.selected ? 'checked' : ''} onchange="toggleFile(${index})">
            </td>
            <td class="col-name" ondblclick="editFileName(${index}, this)">${escapeHtml(file.name)}</td>
            <td class="col-path" ondblclick="editFilePath(${index}, this)">${escapeHtml(file.path)}</td>
            <td class="col-lang">
                <span class="lang-badge">${escapeHtml(file.language)}</span>
            </td>
            <td class="col-size">${formatSize(file.size)}</td>
            <td class="col-actions">
                <button class="action-btn" onclick="event.stopPropagation(); editFileRename(${index})" data-tooltip="${t('rename')}">${t('rename')}</button>
                <button class="action-btn delete" onclick="event.stopPropagation(); deleteFile(${index})" data-tooltip="${t('delete')}">${t('delete')}</button>
            </td>
        `;

        tr.addEventListener('click', () => previewFile(file.id));
        tbody.appendChild(tr);
    });

    updateSelectAll();
    updateStatusBar();
}

function toggleFile(index) {
    extractedFiles[index].selected = !extractedFiles[index].selected;
    updateSelectAll();
    updateStatusBar();
}

function toggleSelectAll() {
    const checked = document.getElementById('select-all').checked;
    extractedFiles.forEach(f => f.selected = checked);
    renderTable();
}

function updateSelectAll() {
    const all = extractedFiles.length > 0 && extractedFiles.every(f => f.selected);
    document.getElementById('select-all').checked = all;
}

function deleteFile(index) {
    extractedFiles.splice(index, 1);
    renderTable();
    if (extractedFiles.length === 0) {
        document.getElementById('preview-content').innerHTML =
            `<div style="padding:20px;color:var(--text-muted);text-align:center;font-size:13px;">${t('status_ready')}</div>`;
    }
}

async function editFileName(index, td) {
    const oldName = extractedFiles[index].name;
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'edit-input';
    input.value = oldName;
    td.textContent = '';
    td.appendChild(input);
    input.focus();
    input.select();

    input.addEventListener('blur', async () => {
        const newName = input.value.trim();
        if (newName && newName !== oldName) {
            const oldEntry = { ...extractedFiles[index] };
            extractedFiles[index].name = newName;
            // Update path if name changed
            const pathParts = extractedFiles[index].path.split('/');
            if (pathParts.length > 0) {
                pathParts[pathParts.length - 1] = newName;
                extractedFiles[index].path = pathParts.join('/');
            }
            try {
                await invoke('update_entry', {
                    oldEntry: oldEntry,
                    newEntry: extractedFiles[index]
                });
            } catch (e) {
                // silently ignore
            }
            renderTable();
        } else {
            renderTable();
        }
    });

    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') input.blur();
        if (e.key === 'Escape') { renderTable(); }
    });
}

function editFilePath(index, td) {
    const oldPath = extractedFiles[index].path;
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'edit-input';
    input.value = oldPath;
    td.textContent = '';
    td.appendChild(input);
    input.focus();
    input.select();

    input.addEventListener('blur', async () => {
        const newPath = input.value.trim();
        if (newPath && newPath !== oldPath) {
            const oldEntry = { ...extractedFiles[index] };
            extractedFiles[index].path = newPath;
            // Update name from new path
            const parts = newPath.split('/');
            extractedFiles[index].name = parts[parts.length - 1];
            // Update language
            const ext = newPath.split('.').pop() || '';
            extractedFiles[index].language = ext || 'text';
            try {
                await invoke('update_entry', {
                    oldEntry: oldEntry,
                    newEntry: extractedFiles[index]
                });
            } catch (e) {
                // silently ignore
            }
            renderTable();
        } else {
            renderTable();
        }
    });

    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') input.blur();
        if (e.key === 'Escape') { renderTable(); }
    });
}

async function editFileRename(index) {
    const oldName = extractedFiles[index].name;
    const newName = prompt(t('enter_new_name'), oldName);
    if (newName && newName.trim() && newName.trim() !== oldName) {
        const oldEntry = { ...extractedFiles[index] };
        extractedFiles[index].name = newName.trim();
        const pathParts = extractedFiles[index].path.split('/');
        if (pathParts.length > 0) {
            pathParts[pathParts.length - 1] = newName.trim();
            extractedFiles[index].path = pathParts.join('/');
        }
        try {
            await invoke('update_entry', {
                oldEntry: oldEntry,
                newEntry: extractedFiles[index]
            });
        } catch (e) {
            // silently ignore
        }
        renderTable();
    }
}

// ========== LEARNING ==========
async function resetLearning() {
    if (!confirm(t('confirm') + '?')) return;

    try {
        // Reset model to defaults
        const defaultModel = {
            patterns: [
                { regex: "(?m)^\\s*//\\s*File:\\s*(.+)$", path_group: 1, content_group: 0, language_hint: null, confidence: 0.9, usage_count: 0 },
                { regex: "(?m)^\\s*#\\s*File:\\s*(.+)$", path_group: 1, content_group: 0, language_hint: null, confidence: 0.9, usage_count: 0 },
                { regex: "\\*\\*File:\\*\\*\\s*(.+?)(?:\\n|$)", path_group: 1, content_group: 0, language_hint: null, confidence: 0.7, usage_count: 0 }
            ],
            feature_weights: [0.125, 0.125, 0.125, 0.125, 0.125, 0.125, 0.125, 0.125],
            training_examples: [],
            version: 1
        };
        await invoke('save_model', { model: defaultModel });
        showStatus(t('learning_model_updated'), 'success');
    } catch (err) {
        showStatus('Error: ' + err.toString(), 'error');
    }
}

// ========== DRAG & DROP ==========
const dropZone = document.getElementById('drop-zone');

dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    e.stopPropagation();
    dropZone.classList.add('dragover');
});

dropZone.addEventListener('dragleave', (e) => {
    e.preventDefault();
    e.stopPropagation();
    dropZone.classList.remove('dragover');
});

dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    e.stopPropagation();
    dropZone.classList.remove('dragover');

    const files = e.dataTransfer.files;
    if (files.length > 0) {
        const file = files[0];
        const reader = new FileReader();
        reader.onload = async (event) => {
            const text = event.target.result;
            try {
                showStatus(t('status_extracting'), 'info');
                const result = await invoke('extract_from_clipboard');
                // Override with the dropped file content
                handleExtractResult(result);
                // If the above doesn't work, do client-side extraction
                if (result.files.length === 0 && text.trim()) {
                    // Basic client-side parse: extract code blocks
                    const clientFiles = clientSideExtract(text);
                    if (clientFiles.length > 0) {
                        extractedFiles = clientFiles;
                        renderTable();
                        document.getElementById('main-content').style.display = 'flex';
                        showStatus(`${t('files_found')}: ${clientFiles.length}`, 'success');
                    }
                }
            } catch (err) {
                showStatus('Error: ' + err.toString(), 'error');
            }
        };
        reader.readAsText(file);
    }
});

// ========== CLIENT-SIDE FALLBACK EXTRACT ==========
function clientSideExtract(text) {
    const files = [];
    const seen = new Set();

    // Match ``` blocks
    const codeBlockRegex = /```(\w+)?(?::(.+?))?\s*\n([\s\S]*?)```/g;
    let match;

    let counter = 1;
    while ((match = codeBlockRegex.exec(text)) !== null) {
        let lang = (match[1] || 'text').toLowerCase();
        let path = match[2] || '';
        let content = (match[3] || '').trim();

        if (!content || seen.has(content)) continue;
        seen.add(content);

        if (!path) {
            const extMap = {
                'rust': 'rs', 'python': 'py', 'javascript': 'js', 'typescript': 'ts',
                'go': 'go', 'ruby': 'rb', 'c': 'c', 'cpp': 'cpp', 'java': 'java',
                'html': 'html', 'css': 'css', 'json': 'json', 'bash': 'sh',
                'sql': 'sql', 'yaml': 'yaml', 'toml': 'toml', 'markdown': 'md',
                'text': 'txt'
            };
            const ext = extMap[lang] || lang;
            path = `code_${counter}.${ext}`;
        }

        const parts = path.split('/');
        const name = parts[parts.length - 1];

        files.push({
            id: crypto.randomUUID ? crypto.randomUUID() : 'id-' + counter,
            path: path,
            name: name,
            language: lang,
            content: content,
            size: content.length,
            selected: true
        });
        counter++;
    }

    return files;
}

// ========== UTILITY ==========
function formatSize(bytes) {
    if (bytes === 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    const val = (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0);
    return `${val} ${units[i]}`;
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function showStatus(text, type) {
    const statusEl = document.getElementById('status-text');
    statusEl.textContent = text;
    statusEl.className = 'status-badge';

    if (type === 'success') {
        statusEl.style.background = 'var(--success)';
    } else if (type === 'error') {
        statusEl.style.background = 'var(--danger)';
    } else if (type === 'warning') {
        statusEl.style.background = 'var(--warning)';
    } else {
        statusEl.style.background = 'var(--accent)';
    }
}

function updateStatusBar() {
    const count = extractedFiles.length;
    const totalSize = extractedFiles.reduce((sum, f) => sum + f.size, 0);
    const selectedCount = extractedFiles.filter(f => f.selected).length;

    document.getElementById('file-count').textContent =
        `${t('files_found')}: ${count}${selectedCount !== count ? ` (${currentLang === 'ru' ? 'выбрано' : 'selected'}: ${selectedCount})` : ''}`;
    document.getElementById('total-size').textContent =
        `${t('total_size')}: ${formatSize(totalSize)}`;
}

// ========== INIT ==========
document.addEventListener('DOMContentLoaded', async () => {
    await loadI18n();
    updateAllTexts();

    // Set version
    try {
        const version = await invoke('get_version');
        document.querySelector('.footer').textContent = `${t('version')}`;
    } catch (e) {
        document.querySelector('.footer').textContent = `v1.0.0`;
    }
});
