<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { projects } from '../stores.js';
  import { fetchProjectFiles } from '../api.js';

  export let task = null;

  const dispatch = createEventDispatcher();
  const isEdit = !!task;

  function todayStr() {
    const d = new Date();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${d.getFullYear()}-${m}-${day}`;
  }

  let project_id = task?.project_id || ($projects.length > 0 ? $projects[0].id : '');
  let name = task?.name || '';
  let description = task?.description || '';
  let priority = task?.priority || 'medium';
  let status = task?.status || 'todo';
  let due_date = task?.due_date || (isEdit ? '' : todayStr());
  let selectedFileIds = (task?.files || []).map((f) => f.id);
  let availableFiles = [];
  let prevProjectId = project_id;

  onMount(() => { loadFilesForProject(project_id); });

  async function loadFilesForProject(pid) {
    if (!pid) { availableFiles = []; return; }
    try {
      availableFiles = await fetchProjectFiles(pid);
    } catch {
      availableFiles = [];
    }
  }

  $: {
    if (project_id !== prevProjectId) {
      selectedFileIds = [];
      prevProjectId = project_id;
    }
    loadFilesForProject(project_id);
  }

  function handleSubmit() {
    if (!name.trim() || !project_id) return;
    dispatch('save', {
      id: task?.id,
      project_id,
      name: name.trim(),
      description: description.trim(),
      priority,
      status,
      due_date,
      file_ids: selectedFileIds,
    });
  }

  function handleDelete() {
    if (confirm('Delete this task?')) {
      dispatch('delete', { id: task.id });
    }
  }

  function handleKeydown(e) {
    if (e.key === 'Escape') dispatch('close');
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="overlay" on:click|self={() => dispatch('close')} on:keydown={() => {}}>
  <div class="modal" role="dialog" aria-modal="true">
    <h2>{isEdit ? 'Edit Task' : 'New Task'}</h2>
    <form on:submit|preventDefault={handleSubmit}>
      <label>
        <span>Project <em>*</em></span>
        <select bind:value={project_id} required>
          <option value="" disabled>Select a project</option>
          {#each $projects as p}
            <option value={p.id}>{p.name} ({p.course})</option>
          {/each}
        </select>
      </label>

      <label>
        <span>Task Name <em>*</em></span>
        <input type="text" bind:value={name} required placeholder="e.g. Write introduction section" />
      </label>

      <label>
        <span>Description</span>
        <textarea bind:value={description} rows="3" placeholder="Optional details..."></textarea>
      </label>

      <div class="row">
        <label class="flex-1">
          <span>Priority</span>
          <select bind:value={priority}>
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
          </select>
        </label>

        <label class="flex-1">
          <span>Status</span>
          <select bind:value={status}>
            <option value="todo">To Do</option>
            <option value="in-progress">In Progress</option>
            <option value="review">Review</option>
            <option value="done">Done</option>
          </select>
        </label>
      </div>

      <label>
        <span>Target finish date</span>
        <input type="date" bind:value={due_date} />
      </label>

      {#if availableFiles.length > 0}
        {@const selectedFiles = selectedFileIds
          .map((id) => availableFiles.find((f) => f.id === id))
          .filter(Boolean)}
        {@const unselectedFiles = availableFiles.filter((f) => !selectedFileIds.includes(f.id))}
        <div class="files-field">
          <span class="files-label">Linked files ({selectedFileIds.length})</span>

          {#if unselectedFiles.length > 0}
            <select
              class="files-add-select"
              value=""
              on:change={(e) => {
                const v = e.target.value;
                if (v) {
                  selectedFileIds = [...selectedFileIds, v];
                  e.target.value = '';
                }
              }}
            >
              <option value="" disabled>+ Add a file…</option>
              {#each unselectedFiles as f (f.id)}
                <option value={f.id}>
                  {f.file_type === 'pdf' ? '\u{1F4C4}' : f.file_type === 'file' ? '\u{1F4CE}' : '\u{1F517}'} {f.name}
                </option>
              {/each}
            </select>
          {/if}

          {#if selectedFiles.length > 0}
            <div class="files-selected-list">
              {#each selectedFiles as f (f.id)}
                <div class="files-selected-item">
                  <span class="files-selected-icon">{f.file_type === 'pdf' ? '\u{1F4C4}' : f.file_type === 'file' ? '\u{1F4CE}' : '\u{1F517}'}</span>
                  <span class="files-selected-name">{f.name}</span>
                  <button
                    type="button"
                    class="files-remove-btn"
                    title="Remove"
                    on:click={() => { selectedFileIds = selectedFileIds.filter((id) => id !== f.id); }}
                  >&#x2715;</button>
                </div>
              {/each}
            </div>
          {:else if unselectedFiles.length > 0}
            <p class="files-empty">No files attached to this task yet.</p>
          {/if}
        </div>
      {/if}

      <div class="actions">
        {#if isEdit}
          <button type="button" class="btn-delete" on:click={handleDelete}>Delete</button>
        {/if}
        <div class="right-actions">
          <button type="button" class="btn-cancel" on:click={() => dispatch('close')}>Cancel</button>
          <button type="submit" class="btn-save">{isEdit ? 'Update' : 'Create'}</button>
        </div>
      </div>
    </form>
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
    padding: 1.75rem;
    width: 100%;
    max-width: 480px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.15);
  }

  h2 {
    margin: 0 0 1.25rem;
    font-size: 1.2rem;
    color: var(--text);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 1rem;
  }

  label span {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  label em { color: #ef4444; font-style: normal; }

  input[type='text'],
  input[type='date'],
  textarea,
  select {
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.875rem;
    font-family: inherit;
    color-scheme: light dark;
  }

  input:focus, textarea:focus, select:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  textarea { resize: vertical; }

  .row { display: flex; gap: 0.75rem; }
  .flex-1 { flex: 1; }

  .files-field { margin-bottom: 1rem; }

  .files-label {
    display: block;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 0.4rem;
  }

  .files-add-select {
    width: 100%;
    margin-bottom: 0.5rem;
  }

  .files-selected-list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.4rem;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    max-height: 180px;
    overflow-y: auto;
  }

  .files-selected-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    background: var(--muted);
    border-radius: 6px;
  }

  .files-selected-icon { flex-shrink: 0; font-size: 0.95rem; }

  .files-selected-name {
    flex: 1;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .files-remove-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0.25rem 0.45rem;
    border-radius: 4px;
    opacity: 0.5;
    transition: all 0.15s;
  }

  .files-remove-btn:hover {
    opacity: 1;
    color: #dc2626;
    background: #fef2f2;
  }

  .files-empty {
    margin: 0;
    padding: 0.5rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
    text-align: center;
  }

  .actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 1.25rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }

  .right-actions { display: flex; gap: 0.5rem; margin-left: auto; }

  button {
    padding: 0.55rem 1.25rem;
    border: none;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-save { background: var(--accent); color: white; }
  .btn-save:hover { opacity: 0.9; }
  .btn-cancel { background: var(--muted); color: var(--text); }
  .btn-cancel:hover { background: var(--border); }
  .btn-delete { background: #fef2f2; color: #dc2626; }
  .btn-delete:hover { background: #fee2e2; }
</style>
