<script>
  import { createEventDispatcher } from 'svelte';
  import Card from './Card.svelte';
  import { tasksByStatus } from '../stores.js';

  export let status;
  export let title;

  const dispatch = createEventDispatcher();
  const items = tasksByStatus(status);

  let dragOver = false;

  function handleDragOver(e) {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    dragOver = true;
  }

  function handleDragLeave() {
    dragOver = false;
  }

  function handleDrop(e) {
    e.preventDefault();
    dragOver = false;
    const taskId = e.dataTransfer.getData('text/plain');
    dispatch('drop', { taskId, status });
  }
</script>

<div
  class="column"
  class:drag-over={dragOver}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
  role="list"
>
  <div class="column-header">
    <h2>{title}</h2>
    <span class="count">{$items.length}</span>
  </div>
  <div class="card-list">
    {#each $items as task (task.id)}
      <Card {task} on:edit />
    {:else}
      <div class="empty">No tasks</div>
    {/each}
  </div>
</div>

<style>
  .column {
    background: var(--column-bg);
    border-radius: 12px;
    padding: 0.75rem;
    min-width: 260px;
    flex: 1;
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 140px);
    transition: background 0.15s;
  }

  .column.drag-over {
    background: var(--accent-light);
  }

  .column-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 0.5rem 0.75rem;
  }

  .column-header h2 {
    font-size: 0.85rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    margin: 0;
  }

  .count {
    background: var(--border);
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.1rem 0.5rem;
    border-radius: 10px;
  }

  .card-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    overflow-y: auto;
    flex: 1;
    padding: 0.25rem;
  }

  .empty {
    color: var(--text-secondary);
    font-size: 0.8rem;
    text-align: center;
    padding: 2rem 0;
  }
</style>
