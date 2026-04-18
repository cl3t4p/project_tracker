<script>
  import { createEventDispatcher } from 'svelte';

  export let project = null;

  const dispatch = createEventDispatcher();
  const isEdit = !!project;

  let name = project?.name || '';
  let course = project?.course || '';
  let description = project?.description || '';
  let deadline = project?.deadline || '';

  function handleSubmit() {
    if (!name.trim() || !course.trim() || !deadline) return;
    dispatch('save', {
      id: project?.id,
      name: name.trim(),
      course: course.trim(),
      description: description.trim(),
      deadline,
    });
  }

  function handleDelete() {
    if (confirm('Delete this project and all its tasks?')) {
      dispatch('delete', { id: project.id });
    }
  }

  function handleAiTasks() {
    dispatch('aiTasks', project);
  }

  function handleKeydown(e) {
    if (e.key === 'Escape') dispatch('close');
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="overlay" on:click|self={() => dispatch('close')} on:keydown={() => {}}>
  <div class="modal" role="dialog" aria-modal="true">
    <h2>{isEdit ? 'Edit Project' : 'New Project'}</h2>
    <form on:submit|preventDefault={handleSubmit}>
      <label>
        <span>Project Name <em>*</em></span>
        <input type="text" bind:value={name} required placeholder="e.g. Database Design Report" />
      </label>

      <label>
        <span>Course <em>*</em></span>
        <input type="text" bind:value={course} required placeholder="e.g. CS301" />
      </label>

      <label>
        <span>Description</span>
        <textarea bind:value={description} rows="3" placeholder="Project details..."></textarea>
      </label>

      <label>
        <span>Deadline <em>*</em></span>
        <input type="date" bind:value={deadline} required />
      </label>

      <div class="actions">
        {#if isEdit}
          <button type="button" class="btn-delete" on:click={handleDelete}>Delete</button>
          <button type="button" class="btn-ai" on:click={handleAiTasks} title="Generate tasks with AI">&#10024; AI tasks</button>
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
    max-width: 440px;
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
  textarea {
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.875rem;
    font-family: inherit;
  }

  input:focus, textarea:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  textarea { resize: vertical; }

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
  .btn-ai {
    background: var(--muted);
    color: var(--text);
    margin-left: 0.5rem;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }
  .btn-ai:hover { background: var(--border); color: var(--accent); }
</style>
