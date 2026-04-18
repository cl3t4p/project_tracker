<script>
  import { createEventDispatcher } from 'svelte';
  import { sidebarOpen, tasksByProject, filterProject } from '../stores.js';

  const dispatch = createEventDispatcher();

  function daysUntilDeadline(deadline) {
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    const dl = new Date(deadline + 'T00:00:00');
    return Math.ceil((dl - now) / (1000 * 60 * 60 * 24));
  }

  function formatDate(dateStr) {
    return new Date(dateStr + 'T00:00:00').toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    });
  }

  function deadlineClass(deadline) {
    const d = daysUntilDeadline(deadline);
    if (d < 0) return 'overdue';
    if (d <= 3) return 'urgent';
    if (d <= 7) return 'soon';
    return 'ok';
  }

  function taskProgress(proj) {
    if (proj.tasks.length === 0) return 0;
    const done = proj.tasks.filter((t) => t.status === 'done').length;
    return Math.round((done / proj.tasks.length) * 100);
  }

  function selectProject(id) {
    $filterProject = $filterProject === id ? '' : id;
  }
</script>

{#if $sidebarOpen}
  <aside class="sidebar">
    <div class="sidebar-header">
      <h2>Projects</h2>
      <div class="sidebar-actions">
        <button class="btn-new" title="New project from PDF" on:click={() => dispatch('newProjectFromPdf')}>&#128196;</button>
        <button class="btn-new" title="New project" on:click={() => dispatch('newProject')}>+</button>
        <button class="btn-close" on:click={() => ($sidebarOpen = false)}>&#x2715;</button>
      </div>
    </div>

    <div class="project-list">
      {#each $tasksByProject as proj (proj.id)}
        {@const days = daysUntilDeadline(proj.deadline)}
        {@const progress = taskProgress(proj)}
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
          class="project-card"
          class:selected={$filterProject === proj.id}
          on:click={() => selectProject(proj.id)}
          on:keydown={(e) => e.key === 'Enter' && selectProject(proj.id)}
          role="button"
          tabindex="0"
        >
          <div class="project-top">
            <button
              class="project-name-btn"
              on:click|stopPropagation={() => dispatch('openProject', proj)}
              title="View project details"
            >{proj.name}</button>
            <div class="card-actions">
              <button
                class="btn-gear"
                on:click|stopPropagation={() => dispatch('openFiles', proj)}
                title="Files &amp; links"
              >&#128206;</button>
              <button
                class="btn-gear"
                on:click|stopPropagation={() => dispatch('aiTasks', proj)}
                title="Generate tasks with AI"
              >&#10024;</button>
              <button
                class="btn-gear"
                on:click|stopPropagation={() => dispatch('editProject', proj)}
                title="Edit project"
              >&#9881;</button>
            </div>
          </div>
          <span class="project-course">{proj.course}</span>

          <div class="project-deadline">
            <span class="deadline-label {deadlineClass(proj.deadline)}">
              {#if days < 0}
                {Math.abs(days)}d overdue
              {:else if days === 0}
                Due today
              {:else if days === 1}
                Due tomorrow
              {:else}
                {days}d left
              {/if}
            </span>
            <span class="deadline-date">{formatDate(proj.deadline)}</span>
          </div>

          <div class="progress-bar">
            <div class="progress-fill" style="width: {progress}%"></div>
          </div>
          <span class="progress-text">
            {proj.tasks.filter((t) => t.status === 'done').length}/{proj.tasks.length} tasks
          </span>
        </div>
      {:else}
        <div class="empty">No projects yet</div>
      {/each}
    </div>
  </aside>
{/if}

<style>
  .sidebar {
    width: 300px;
    min-width: 300px;
    background: var(--surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    height: calc(100vh - 57px);
    overflow: hidden;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .sidebar-header h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--text);
  }

  .sidebar-actions {
    display: flex;
    gap: 0.25rem;
  }

  .btn-new, .btn-close {
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
    transition: all 0.15s;
  }

  .btn-new:hover, .btn-close:hover {
    background: var(--muted);
    color: var(--text);
  }

  .project-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .project-card {
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .project-card:hover {
    background: var(--muted);
  }

  .project-card.selected {
    border-color: var(--accent);
    background: var(--accent-light);
  }

  .project-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .project-name-btn {
    font-weight: 600;
    font-size: 0.875rem;
    color: var(--text);
    margin-bottom: 0.2rem;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    transition: color 0.15s;
  }

  .project-name-btn:hover {
    color: var(--accent);
  }

  .btn-gear {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 1rem;
    padding: 0;
    line-height: 1;
    opacity: 0.4;
    transition: opacity 0.15s, color 0.15s;
  }

  .btn-gear:hover {
    opacity: 1;
    color: var(--accent);
  }

  .card-actions {
    display: flex;
    gap: 0.3rem;
  }


  .project-course {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
    display: block;
    margin-bottom: 0.5rem;
  }

  .project-deadline {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
    font-size: 0.75rem;
  }

  .deadline-label {
    font-weight: 600;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
  }

  .deadline-label.overdue { color: #dc2626; background: #fef2f2; }
  .deadline-label.urgent  { color: #ea580c; background: #fff7ed; }
  .deadline-label.soon    { color: #ca8a04; background: #fefce8; }
  .deadline-label.ok      { color: #16a34a; background: #f0fdf4; }

  .deadline-date {
    color: var(--text-secondary);
  }

  .progress-bar {
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 0.3rem;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.3s;
  }

  .progress-text {
    font-size: 0.7rem;
    color: var(--text-secondary);
  }

  .empty {
    color: var(--text-secondary);
    font-size: 0.8rem;
    text-align: center;
    padding: 2rem 0;
  }
</style>
