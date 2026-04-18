<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { tasks, projectFiles } from '../stores.js';
  import { fetchProjectFiles, createProjectFile, uploadProjectFile, deleteProjectFile, fileToBase64 } from '../api.js';

  export let project;

  const dispatch = createEventDispatcher();

  let showAddLink = false;
  let showAddPdf = false;
  let linkName = '';
  let linkUrl = '';
  let pdfName = '';
  let pdfFile = null;
  let uploading = false;

  $: projectTasks = $tasks.filter(t => t.project_id === project.id);
  $: doneTasks = projectTasks.filter(t => t.status === 'done').length;
  $: progress = projectTasks.length > 0 ? Math.round((doneTasks / projectTasks.length) * 100) : 0;
  $: links = $projectFiles.filter(f => f.file_type === 'link');
  $: pdfs = $projectFiles.filter(f => f.file_type === 'pdf');

  onMount(loadFiles);

  async function loadFiles() {
    $projectFiles = await fetchProjectFiles(project.id);
  }

  function daysUntilDeadline(deadline) {
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    const dl = new Date(deadline + 'T00:00:00');
    return Math.ceil((dl - now) / (1000 * 60 * 60 * 24));
  }

  function formatDate(dateStr) {
    return new Date(dateStr + 'T00:00:00').toLocaleDateString('en-GB', {
      day: 'numeric', month: 'short', year: 'numeric',
    });
  }

  function deadlineClass(deadline) {
    const d = daysUntilDeadline(deadline);
    if (d < 0) return 'overdue';
    if (d <= 3) return 'urgent';
    if (d <= 7) return 'soon';
    return 'ok';
  }

  async function handleAddLink() {
    if (!linkName.trim() || !linkUrl.trim()) return;
    await createProjectFile({
      project_id: project.id,
      name: linkName.trim(),
      file_type: 'link',
      url: linkUrl.trim(),
    });
    linkName = '';
    linkUrl = '';
    showAddLink = false;
    await loadFiles();
  }

  async function handleAddPdf() {
    if (!pdfName.trim() || !pdfFile) return;
    uploading = true;
    try {
      const b64 = await fileToBase64(pdfFile);
      await uploadProjectFile(project.id, pdfName.trim(), b64);
      pdfName = '';
      pdfFile = null;
      showAddPdf = false;
      await loadFiles();
    } finally {
      uploading = false;
    }
  }

  async function handleDeleteFile(file) {
    if (!confirm(`Remove "${file.name}"?`)) return;
    await deleteProjectFile(file.id);
    await loadFiles();
  }

  function statusLabel(status) {
    return { 'todo': 'To Do', 'in-progress': 'In Progress', 'review': 'Review', 'done': 'Done' }[status] || status;
  }

  function priorityLabel(p) {
    return { low: 'Low', medium: 'Medium', high: 'High' }[p] || p;
  }
</script>

<div class="detail-page">
  <div class="detail-header">
    <button class="btn-back" on:click={() => dispatch('back')}>&#8592; Back</button>
    <div class="header-actions">
      <button class="btn-edit" on:click={() => dispatch('editProject', project)}>Edit Project</button>
    </div>
  </div>

  <div class="detail-content">
    <!-- Project Info -->
    <section class="info-section">
      <div class="project-title-row">
        <h1>{project.name}</h1>
        <span class="course-badge">{project.course}</span>
      </div>

      {#if project.description}
        <p class="project-description">{project.description}</p>
      {/if}

      <div class="meta-row">
        <div class="meta-item">
          <span class="meta-label">Deadline</span>
          <span class="meta-value deadline-tag {deadlineClass(project.deadline)}">
            {formatDate(project.deadline)}
            {#if daysUntilDeadline(project.deadline) < 0}
              ({Math.abs(daysUntilDeadline(project.deadline))}d overdue)
            {:else if daysUntilDeadline(project.deadline) === 0}
              (Due today)
            {:else}
              ({daysUntilDeadline(project.deadline)}d left)
            {/if}
          </span>
        </div>
        <div class="meta-item">
          <span class="meta-label">Progress</span>
          <div class="progress-row">
            <div class="progress-bar"><div class="progress-fill" style="width: {progress}%"></div></div>
            <span class="progress-text">{doneTasks}/{projectTasks.length} tasks ({progress}%)</span>
          </div>
        </div>
      </div>
    </section>

    <div class="two-columns">
      <!-- Links Section -->
      <section class="files-section">
        <div class="section-header">
          <h2>Links</h2>
          <button class="btn-add-small" on:click={() => { showAddLink = !showAddLink; showAddPdf = false; }}>+ Add Link</button>
        </div>

        {#if showAddLink}
          <form class="add-form" on:submit|preventDefault={handleAddLink}>
            <input type="text" bind:value={linkName} placeholder="Link name" required />
            <input type="url" bind:value={linkUrl} placeholder="https://..." required />
            <div class="form-actions">
              <button type="button" class="btn-cancel-small" on:click={() => { showAddLink = false; }}>Cancel</button>
              <button type="submit" class="btn-save-small">Add</button>
            </div>
          </form>
        {/if}

        <div class="file-list">
          {#each links as link (link.id)}
            <div class="file-item">
              <span class="file-icon">&#128279;</span>
              <a href={link.url} target="_blank" rel="noopener noreferrer" class="file-name">{link.name}</a>
              <button class="btn-remove" on:click={() => handleDeleteFile(link)} title="Remove">&#x2715;</button>
            </div>
          {:else}
            <p class="empty-text">No links added yet</p>
          {/each}
        </div>
      </section>

      <!-- PDFs Section -->
      <section class="files-section">
        <div class="section-header">
          <h2>PDF Attachments</h2>
          <button class="btn-add-small" on:click={() => { showAddPdf = !showAddPdf; showAddLink = false; }}>+ Upload PDF</button>
        </div>

        {#if showAddPdf}
          <form class="add-form" on:submit|preventDefault={handleAddPdf}>
            <input type="text" bind:value={pdfName} placeholder="PDF name" required />
            <input type="file" accept=".pdf" on:change={e => pdfFile = e.target.files[0]} required />
            <div class="form-actions">
              <button type="button" class="btn-cancel-small" on:click={() => { showAddPdf = false; }}>Cancel</button>
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
              <button class="btn-remove" on:click={() => handleDeleteFile(pdf)} title="Remove">&#x2715;</button>
            </div>
          {:else}
            <p class="empty-text">No PDFs attached yet</p>
          {/each}
        </div>
      </section>
    </div>

    <!-- Tasks Overview -->
    <section class="tasks-section">
      <div class="section-header">
        <h2>Tasks ({projectTasks.length})</h2>
      </div>
      <div class="tasks-table">
        {#each projectTasks as t (t.id)}
          {@const linkedFile = t.file_id ? $projectFiles.find(f => f.id === t.file_id) : null}
          <div class="task-row" class:done={t.status === 'done'}>
            <span class="task-priority priority-{t.priority}" title="{priorityLabel(t.priority)} priority"></span>
            <div class="task-info">
              <span class="task-name">{t.name}</span>
              {#if t.description}
                <span class="task-desc">{t.description}</span>
              {/if}
              {#if linkedFile}
                <a
                  href={linkedFile.file_type === 'link' ? linkedFile.url : linkedFile.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="task-file-link"
                >
                  {linkedFile.file_type === 'pdf' ? '&#128196;' : '&#128279;'} {linkedFile.name}
                </a>
              {/if}
            </div>
            <span class="task-status status-{t.status}">{statusLabel(t.status)}</span>
            {#if t.due_date}
              <span class="task-date">{formatDate(t.due_date)}</span>
            {/if}
          </div>
        {:else}
          <p class="empty-text">No tasks yet</p>
        {/each}
      </div>
    </section>
  </div>
</div>

<style>
  .detail-page {
    flex: 1;
    overflow-y: auto;
    height: calc(100vh - 57px);
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 2rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .btn-back {
    background: none;
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    cursor: pointer;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    font-weight: 600;
    transition: all 0.15s;
  }

  .btn-back:hover {
    background: var(--muted);
  }

  .btn-edit {
    padding: 0.5rem 1.25rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .btn-edit:hover { opacity: 0.9; }

  .detail-content {
    padding: 2rem;
    max-width: 960px;
    margin: 0 auto;
  }

  .info-section {
    margin-bottom: 2rem;
  }

  .project-title-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
  }

  h1 {
    margin: 0;
    font-size: 1.75rem;
    font-weight: 800;
    color: var(--text);
  }

  .course-badge {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 0.25rem 0.75rem;
    border-radius: 20px;
    background: var(--accent-light);
    color: var(--accent);
  }

  .project-description {
    color: var(--text-secondary);
    font-size: 0.95rem;
    line-height: 1.6;
    margin: 0 0 1.25rem;
  }

  .meta-row {
    display: flex;
    gap: 2.5rem;
    flex-wrap: wrap;
  }

  .meta-item {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .meta-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .meta-value {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text);
  }

  .deadline-tag {
    padding: 0.2rem 0.6rem;
    border-radius: 6px;
    display: inline-block;
  }

  .deadline-tag.overdue { color: #dc2626; background: #fef2f2; }
  .deadline-tag.urgent  { color: #ea580c; background: #fff7ed; }
  .deadline-tag.soon    { color: #ca8a04; background: #fefce8; }
  .deadline-tag.ok      { color: #16a34a; background: #f0fdf4; }

  .progress-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .progress-bar {
    width: 120px;
    height: 6px;
    background: var(--border);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.3s;
  }

  .progress-text {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .two-columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
    margin-bottom: 2rem;
  }

  @media (max-width: 700px) {
    .two-columns { grid-template-columns: 1fr; }
  }

  .files-section, .tasks-section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1.25rem;
  }

  .tasks-section {
    margin-bottom: 2rem;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .section-header h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--text);
  }

  .btn-add-small {
    padding: 0.35rem 0.85rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
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
  .add-form input[type='url'] {
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.85rem;
    font-family: inherit;
  }

  .add-form input[type='file'] {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .add-form input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  .form-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  .btn-save-small {
    padding: 0.35rem 0.85rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-save-small:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-cancel-small {
    padding: 0.35rem 0.85rem;
    background: var(--muted);
    color: var(--text);
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-cancel-small:hover { background: var(--border); }

  .file-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.6rem 0.75rem;
    background: var(--muted);
    border-radius: 8px;
    transition: background 0.15s;
  }

  .file-item:hover {
    background: var(--border);
  }

  .file-icon {
    font-size: 1.1rem;
    flex-shrink: 0;
  }

  .file-name {
    flex: 1;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--accent);
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-name:hover { text-decoration: underline; }

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
    font-size: 0.85rem;
    text-align: center;
    padding: 1rem 0;
    margin: 0;
  }

  /* Tasks table */
  .tasks-table {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: var(--muted);
    border-radius: 8px;
    transition: background 0.15s;
  }

  .task-row:hover { background: var(--border); }
  .task-row.done { opacity: 0.6; }

  .task-priority {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .task-priority.priority-high { background: #ef4444; }
  .task-priority.priority-medium { background: #f59e0b; }
  .task-priority.priority-low { background: #22c55e; }

  .task-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .task-name {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .task-desc {
    font-size: 0.8rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .task-file-link {
    font-size: 0.75rem;
    color: var(--accent);
    text-decoration: none;
  }

  .task-file-link:hover { text-decoration: underline; }

  .task-status {
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.2rem 0.6rem;
    border-radius: 20px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .task-status.status-todo { color: var(--text-secondary); background: var(--muted); border: 1px solid var(--border); }
  .task-status.status-in-progress { color: #2563eb; background: #eff6ff; }
  .task-status.status-review { color: #9333ea; background: #faf5ff; }
  .task-status.status-done { color: #16a34a; background: #f0fdf4; }

  .task-date {
    font-size: 0.8rem;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }
</style>
