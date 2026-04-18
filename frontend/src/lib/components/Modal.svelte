<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { projects } from '../stores.js';
  import { fetchProjectFiles } from '../api.js';

  export let task = null;

  const dispatch = createEventDispatcher();
  const isEdit = !!task;

  let project_id = task?.project_id || ($projects.length > 0 ? $projects[0].id : '');
  let name = task?.name || '';
  let description = task?.description || '';
  let priority = task?.priority || 'medium';
  let status = task?.status || 'todo';
  let due_date = task?.due_date || '';
  let file_id = task?.file_id || '';
  let availableFiles = [];

  onMount(() => { loadFilesForProject(project_id); });

  async function loadFilesForProject(pid) {
    if (!pid) { availableFiles = []; return; }
    try {
      availableFiles = await fetchProjectFiles(pid);
    } catch {
      availableFiles = [];
    }
  }

  $: loadFilesForProject(project_id);

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
      file_id: file_id || '',
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
        <label>
          <span>Linked file</span>
          <select bind:value={file_id}>
            <option value="">None</option>
            {#each availableFiles as f}
              <option value={f.id}>{f.file_type === 'pdf' ? '\u{1F4C4}' : '\u{1F517}'} {f.name}</option>
            {/each}
          </select>
        </label>
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
  textarea,
  select {
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.875rem;
    font-family: inherit;
  }

  input:focus, textarea:focus, select:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  textarea { resize: vertical; }

  .row { display: flex; gap: 0.75rem; }
  .flex-1 { flex: 1; }

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
