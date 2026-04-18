<script>
  import { createEventDispatcher } from 'svelte';
  import { projects } from '../stores.js';

  export let task;

  const dispatch = createEventDispatcher();

  $: project = $projects.find((p) => p.id === task.project_id);
  $: taskFiles = task.files || [];

  function openFile(f) {
    if (f && f.url) window.open(f.url, '_blank', 'noopener');
  }

  function daysUntilDeadline(deadline) {
    if (!deadline) return null;
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    const dl = new Date(deadline + 'T00:00:00');
    return Math.ceil((dl - now) / (1000 * 60 * 60 * 24));
  }

  function formatDate(dateStr) {
    if (!dateStr) return '';
    return new Date(dateStr + 'T00:00:00').toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
    });
  }

  $: deadline = task.due_date || project?.deadline;
  $: isTaskDate = !!task.due_date;
  $: daysLeft = deadline ? daysUntilDeadline(deadline) : null;
  $: deadlineClass =
    task.status === 'done'
      ? 'done'
      : daysLeft === null
        ? 'ok'
        : daysLeft < 0
          ? 'overdue'
          : daysLeft <= 3
            ? 'urgent'
            : daysLeft <= 7
              ? 'soon'
              : 'ok';

  function handleDragStart(e) {
    e.dataTransfer.setData('text/plain', task.id);
    e.dataTransfer.effectAllowed = 'move';
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="card priority-{task.priority}"
  draggable="true"
  on:dragstart={handleDragStart}
  on:click={() => dispatch('edit', task)}
  on:keydown={(e) => e.key === 'Enter' && dispatch('edit', task)}
  role="button"
  tabindex="0"
>
  <div class="card-header">
    {#if project}
      <span class="project-badge">{project.name}</span>
    {/if}
    <span class="priority-dot" title="{task.priority} priority"></span>
  </div>
  <h3 class="card-title">{task.name}</h3>
  {#if task.description}
    <p class="card-desc">{task.description}</p>
  {/if}
  {#if taskFiles.length > 0}
    <div class="file-chips">
      {#each taskFiles as f (f.id)}
        <button
          type="button"
          class="file-chip"
          title="Open {f.name}"
          on:click|stopPropagation={() => openFile(f)}
          on:keydown|stopPropagation={(e) => { if (e.key === 'Enter' || e.key === ' ') openFile(f); }}
        >
          <span class="chip-icon">{f.file_type === 'pdf' ? '\u{1F4C4}' : '\u{1F517}'}</span>
          <span class="chip-label">{f.name}</span>
        </button>
      {/each}
    </div>
  {/if}
  {#if deadline}
    <div class="card-footer">
      <span class="deadline {deadlineClass}">
        {#if task.status === 'done'}
          Completed
        {:else if daysLeft < 0}
          {Math.abs(daysLeft)}d overdue
        {:else if daysLeft === 0}
          Due today
        {:else if daysLeft === 1}
          Due tomorrow
        {:else}
          {daysLeft}d left
        {/if}
      </span>
      <span class="date" title={isTaskDate ? 'Task target date' : 'Project deadline'}>
        {isTaskDate ? '' : '~ '}{formatDate(deadline)}
      </span>
    </div>
  {/if}
</div>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1rem 1.1rem;
    cursor: grab;
    transition: box-shadow 0.15s, transform 0.15s;
    border-left: 3px solid var(--border);
  }

  .card:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    transform: translateY(-1px);
  }

  .card:active {
    cursor: grabbing;
  }

  .card.priority-high {
    border-left-color: #ef4444;
  }
  .card.priority-medium {
    border-left-color: #f59e0b;
  }
  .card.priority-low {
    border-left-color: #22c55e;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .project-badge {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 0.15rem 0.5rem;
    border-radius: 20px;
    background: var(--accent-light);
    color: var(--accent);
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .priority-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .priority-high .priority-dot {
    background: #ef4444;
  }
  .priority-medium .priority-dot {
    background: #f59e0b;
  }
  .priority-low .priority-dot {
    background: #22c55e;
  }

  .card-title {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0 0 0.4rem;
    color: var(--text);
    line-height: 1.3;
  }

  .card-desc {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin: 0 0 0.5rem;
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
  }

  .deadline {
    font-weight: 600;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
  }

  .deadline.overdue { color: #dc2626; background: #fef2f2; }
  .deadline.urgent  { color: #ea580c; background: #fff7ed; }
  .deadline.soon    { color: #ca8a04; background: #fefce8; }
  .deadline.ok      { color: #16a34a; background: #f0fdf4; }
  .deadline.done    { color: var(--text-secondary); background: var(--muted); }

  .date { color: var(--text-secondary); }

  .file-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-bottom: 0.45rem;
  }

  .file-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    max-width: 100%;
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-light);
    border: none;
    border-radius: 4px;
    padding: 0.15rem 0.45rem;
    cursor: pointer;
    font-family: inherit;
    transition: opacity 0.15s;
  }

  .file-chip:hover { opacity: 0.8; }

  .chip-icon { flex-shrink: 0; }

  .chip-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 160px;
  }
</style>
