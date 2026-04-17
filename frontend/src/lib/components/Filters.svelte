<script>
  import { projects, filterProject, filterPriority, filterSearch } from '../stores.js';
</script>

<div class="filters">
  <input
    type="text"
    placeholder="Search tasks..."
    bind:value={$filterSearch}
    class="filter-input"
  />

  <select bind:value={$filterProject} class="filter-select">
    <option value="">All Projects</option>
    {#each $projects as p}
      <option value={p.id}>{p.name}</option>
    {/each}
  </select>

  <select bind:value={$filterPriority} class="filter-select">
    <option value="">All Priorities</option>
    <option value="low">Low</option>
    <option value="medium">Medium</option>
    <option value="high">High</option>
  </select>

  {#if $filterProject || $filterPriority || $filterSearch}
    <button
      class="clear-btn"
      on:click={() => {
        $filterProject = '';
        $filterPriority = '';
        $filterSearch = '';
      }}
    >
      Clear
    </button>
  {/if}
</div>

<style>
  .filters {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .filter-input {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.875rem;
    min-width: 180px;
  }

  .filter-select {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.875rem;
    cursor: pointer;
  }

  .filter-input:focus,
  .filter-select:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  .clear-btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    background: var(--muted);
    color: var(--text);
    font-size: 0.875rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  .clear-btn:hover {
    background: var(--border);
  }
</style>
