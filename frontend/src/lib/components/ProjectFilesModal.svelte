<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { projectFiles } from '../stores.js';
  import {
    fetchProjectFiles, createProjectFile, uploadProjectFile, deleteProjectFile,
    updateProjectFile, fetchSubsections, fileToBase64,
  } from '../api.js';

  let subsections = [];

  const CATEGORIES = [
    { key: 'course',     title: 'Course material' },
    { key: 'assignment', title: 'Assignment' },
    { key: 'lab',        title: 'Lab' },
    { key: 'exam',       title: 'Exam prep' },
    { key: 'other',      title: 'Other' },
  ];

  export let project;

  const dispatch = createEventDispatcher();

  let showAddLink = false;
  let showAddPdf = false;
  let showAddFile = false;
  let linkName = '';
  let linkUrl = '';
  let linkCategory = 'other';
  let linkSub = '';
  let pdfName = '';
  let pdfFile = null;
  let pdfCategory = 'course';
  let pdfSub = '';
  let fileName = '';
  let fileBlob = null;
  let fileCategory = 'other';
  let fileSub = '';
  let uploading = false;
  let uploadingFile = false;

  $: links = $projectFiles.filter(f => f.file_type === 'link');
  $: pdfs = $projectFiles.filter(f => f.file_type === 'pdf');
  $: genericFiles = $projectFiles.filter(f => f.file_type === 'file');

  onMount(loadFiles);

  async function loadFiles() {
    $projectFiles = await fetchProjectFiles(project.id);
    subsections = await fetchSubsections();
  }

  function subsFor(catKey) {
    return subsections.filter((s) => s.category === catKey).map((s) => s.name);
  }

  async function handleAddLink() {
    if (!linkName.trim() || !linkUrl.trim()) return;
    await createProjectFile({
      project_id: project.id,
      name: linkName.trim(),
      file_type: 'link',
      url: linkUrl.trim(),
      category: linkCategory,
    });
    linkName = '';
    linkUrl = '';
    linkCategory = 'other';
    showAddLink = false;
    await loadFiles();
  }

  async function handleAddPdf() {
    if (!pdfName.trim() || !pdfFile) return;
    uploading = true;
    try {
      const b64 = await fileToBase64(pdfFile);
      await uploadProjectFile(project.id, pdfName.trim(), b64, pdfFile.name, pdfCategory);
      pdfName = '';
      pdfFile = null;
      pdfCategory = 'course';
      showAddPdf = false;
      await loadFiles();
    } finally {
      uploading = false;
    }
  }

  async function handleAddFile() {
    if (!fileBlob) return;
    uploadingFile = true;
    try {
      const b64 = await fileToBase64(fileBlob);
      const displayName = fileName.trim() || fileBlob.name;
      await uploadProjectFile(project.id, displayName, b64, fileBlob.name, fileCategory);
      fileName = '';
      fileBlob = null;
      fileCategory = 'other';
      showAddFile = false;
      await loadFiles();
    } finally {
      uploadingFile = false;
    }
  }

  async function changeCategory(file, category) {
    if (category === file.category) return;
    await updateProjectFile(file.id, { category, subsection: null });
    await loadFiles();
  }

  async function changeSubsection(file, sub) {
    await updateProjectFile(file.id, { subsection: sub || null });
    await loadFiles();
  }

  async function handleDeleteFile(file) {
    if (!confirm(`Remove "${file.name}"?`)) return;
    await deleteProjectFile(file.id);
    await loadFiles();
  }

  function handleKeydown(e) {
    if (e.key === 'Escape') dispatch('close');
  }

  function fileExt(f) {
    const src = f.url || f.name || '';
    const i = src.lastIndexOf('.');
    if (i < 0 || i === src.length - 1) return '';
    const ext = src.slice(i + 1).split(/[?#]/)[0].toLowerCase();
    return /^[a-z0-9]{1,8}$/.test(ext) ? ext : '';
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="overlay" on:click|self={() => dispatch('close')} on:keydown={() => {}}>
  <div class="modal" role="dialog" aria-modal="true">
    <div class="modal-header">
      <div>
        <h2>{project.name}</h2>
        <p class="subtitle">Resource library &middot; links &amp; PDFs available to this project's tasks</p>
      </div>
      <button class="btn-close" on:click={() => dispatch('close')} title="Close">&#x2715;</button>
    </div>

    <div class="sections">
      <section class="files-section">
        <div class="section-header">
          <h3>&#128279; Links</h3>
          <button class="btn-add-small" on:click={() => { showAddLink = !showAddLink; showAddPdf = false; showAddFile = false; }}>
            {showAddLink ? 'Cancel' : '+ Add Link'}
          </button>
        </div>

        {#if showAddLink}
          <form class="add-form" on:submit|preventDefault={handleAddLink}>
            <input type="text" bind:value={linkName} placeholder="Link name" required />
            <input type="url" bind:value={linkUrl} placeholder="https://..." required />
            <select bind:value={linkCategory}>
              {#each CATEGORIES as c}<option value={c.key}>{c.title}</option>{/each}
            </select>
            <div class="form-actions">
              <button type="submit" class="btn-save-small">Add</button>
            </div>
          </form>
        {/if}

        <div class="file-list">
          {#each links as link (link.id)}
            <div class="file-item">
              <span class="file-icon">&#128279;</span>
              <a href={link.url} target="_blank" rel="noopener noreferrer" class="file-name">{link.name}</a>
              <select class="cat-inline" value={link.category || 'other'} on:change={(e) => changeCategory(link, e.target.value)} title="Category">
                {#each CATEGORIES as c}<option value={c.key}>{c.title}</option>{/each}
              </select>
              <button class="btn-remove" on:click={() => handleDeleteFile(link)} title="Remove">&#x2715;</button>
            </div>
          {:else}
            <p class="empty-text">No links yet</p>
          {/each}
        </div>
      </section>

      <section class="files-section">
        <div class="section-header">
          <h3>&#128196; PDF Attachments</h3>
          <button class="btn-add-small" on:click={() => { showAddPdf = !showAddPdf; showAddLink = false; showAddFile = false; }}>
            {showAddPdf ? 'Cancel' : '+ Upload PDF'}
          </button>
        </div>

        {#if showAddPdf}
          <form class="add-form" on:submit|preventDefault={handleAddPdf}>
            <input type="text" bind:value={pdfName} placeholder="PDF name" required />
            <input type="file" accept=".pdf" on:change={e => pdfFile = e.target.files[0]} required />
            <select bind:value={pdfCategory}>
              {#each CATEGORIES as c}<option value={c.key}>{c.title}</option>{/each}
            </select>
            <div class="form-actions">
              <button type="submit" class="btn-save-small" disabled={uploading}>
                {uploading ? 'Uploading...' : 'Upload'}
              </button>
            </div>
          </form>
        {/if}

        <div class="file-list">
          {#each pdfs as pdf (pdf.id)}
            <div class="file-item">
              <span class="file-icon">&#128196;</span>
              <a href={pdf.url} target="_blank" rel="noopener noreferrer" class="file-name">{pdf.name}</a>
              {#if fileExt(pdf)}<span class="file-ext">.{fileExt(pdf)}</span>{/if}
              <select class="cat-inline" value={pdf.category || 'other'} on:change={(e) => changeCategory(pdf, e.target.value)} title="Category">
                {#each CATEGORIES as c}<option value={c.key}>{c.title}</option>{/each}
              </select>
              <button class="btn-remove" on:click={() => handleDeleteFile(pdf)} title="Remove">&#x2715;</button>
            </div>
          {:else}
            <p class="empty-text">No PDFs yet</p>
          {/each}
        </div>
      </section>

      <section class="files-section">
        <div class="section-header">
          <h3>&#128206; Files</h3>
          <button class="btn-add-small" on:click={() => { showAddFile = !showAddFile; showAddLink = false; showAddPdf = false; }}>
            {showAddFile ? 'Cancel' : '+ Upload File'}
          </button>
        </div>

        {#if showAddFile}
          <form class="add-form" on:submit|preventDefault={handleAddFile}>
            <input type="text" bind:value={fileName} placeholder="Display name (optional)" />
            <input type="file" on:change={e => fileBlob = e.target.files[0]} required />
            <select bind:value={fileCategory}>
              {#each CATEGORIES as c}<option value={c.key}>{c.title}</option>{/each}
            </select>
            <div class="form-actions">
              <button type="submit" class="btn-save-small" disabled={uploadingFile}>
                {uploadingFile ? 'Uploading...' : 'Upload'}
              </button>
            </div>
          </form>
        {/if}

        <div class="file-list">
          {#each genericFiles as file (file.id)}
            <div class="file-item">
              <span class="file-icon">&#128206;</span>
              <a href={file.url} target="_blank" rel="noopener noreferrer" class="file-name">{file.name}</a>
              {#if fileExt(file)}<span class="file-ext">.{fileExt(file)}</span>{/if}
              <select class="cat-inline" value={file.category || 'other'} on:change={(e) => changeCategory(file, e.target.value)} title="Category">
                {#each CATEGORIES as c}<option value={c.key}>{c.title}</option>{/each}
              </select>
              <button class="btn-remove" on:click={() => handleDeleteFile(file)} title="Remove">&#x2715;</button>
            </div>
          {:else}
            <p class="empty-text">No files yet</p>
          {/each}
        </div>
      </section>
    </div>

    <div class="modal-footer">
      <p class="hint">Tip: open any task and pick one of these from the <strong>Linked file</strong> dropdown.</p>
      <button class="btn-done" on:click={() => dispatch('close')}>Done</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1rem;
  }

  .modal {
    background: var(--surface);
    border-radius: 16px;
    width: 100%;
    max-width: 640px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
    overflow: hidden;
  }

  .modal-header {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }

  .modal-header h2 {
    margin: 0 0 0.25rem;
    font-size: 1.1rem;
    color: var(--text);
  }

  .subtitle {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-secondary);
  }

  .btn-close {
    background: none;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.85rem;
    flex-shrink: 0;
  }

  .btn-close:hover { background: var(--muted); color: var(--text); }

  .sections {
    padding: 1.25rem 1.5rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .files-section {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1rem;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .section-header h3 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 700;
    color: var(--text);
  }

  .btn-add-small {
    padding: 0.3rem 0.75rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-add-small:hover { opacity: 0.9; }

  .add-form {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--muted);
    border-radius: 8px;
    margin-bottom: 0.75rem;
  }

  .add-form input[type='text'],
  .add-form input[type='url'],
  .add-form select {
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.85rem;
    font-family: inherit;
  }

  .cat-inline {
    padding: 0.2rem 0.35rem;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg);
    color: var(--text-secondary);
    font-size: 0.7rem;
    cursor: pointer;
    flex-shrink: 0;
  }

  .add-form input[type='file'] {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .add-form input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
  }

  .btn-save-small {
    padding: 0.35rem 0.9rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-save-small:disabled { opacity: 0.5; cursor: not-allowed; }

  .file-list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.55rem 0.75rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .file-item:hover { background: var(--muted); }

  .file-icon { font-size: 1.05rem; flex-shrink: 0; }

  .file-name {
    flex: 1;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--accent);
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-name:hover { text-decoration: underline; }

  .file-ext {
    flex-shrink: 0;
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--muted);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.1rem 0.4rem;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .btn-remove {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    opacity: 0.4;
    transition: all 0.15s;
  }

  .btn-remove:hover {
    opacity: 1;
    color: #dc2626;
    background: #fef2f2;
  }

  .empty-text {
    color: var(--text-secondary);
    font-size: 0.82rem;
    text-align: center;
    padding: 0.75rem 0;
    margin: 0;
  }

  .modal-footer {
    padding: 0.9rem 1.5rem;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    background: var(--surface);
  }

  .hint {
    margin: 0;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .btn-done {
    padding: 0.5rem 1.1rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-done:hover { opacity: 0.9; }
</style>
