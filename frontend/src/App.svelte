<script>
  import { onMount } from 'svelte';
  import { projects, tasks, courses, sidebarOpen } from './lib/stores.js';
  import {
    fetchProjects, fetchTasks, fetchCourses,
    createProject, updateProject, deleteProject,
    createTask, updateTask, deleteTask,
  } from './lib/api.js';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Filters from './lib/components/Filters.svelte';
  import Column from './lib/components/Column.svelte';
  import Modal from './lib/components/Modal.svelte';
  import ProjectModal from './lib/components/ProjectModal.svelte';

  let showTaskModal = false;
  let editingTask = null;
  let showProjectModal = false;
  let editingProject = null;

  const columns = [
    { status: 'todo', title: 'To Do' },
    { status: 'in-progress', title: 'In Progress' },
    { status: 'review', title: 'Review' },
    { status: 'done', title: 'Done' },
  ];

  onMount(loadData);

  async function loadData() {
    $projects = await fetchProjects();
    $tasks = await fetchTasks();
    $courses = await fetchCourses();
  }

  // Task handlers
  function openNewTask() {
    if ($projects.length === 0) {
      alert('Create a project first!');
      editingProject = null;
      showProjectModal = true;
      return;
    }
    editingTask = null;
    showTaskModal = true;
  }

  function openEditTask(e) {
    editingTask = e.detail;
    showTaskModal = true;
  }

  async function handleSaveTask(e) {
    const data = e.detail;
    if (data.id) {
      await updateTask(data.id, data);
    } else {
      await createTask(data);
    }
    showTaskModal = false;
    await loadData();
  }

  async function handleDeleteTask(e) {
    await deleteTask(e.detail.id);
    showTaskModal = false;
    await loadData();
  }

  async function handleDrop(e) {
    const { taskId, status } = e.detail;
    await updateTask(taskId, { status });
    await loadData();
  }

  // Project handlers
  function openNewProject() {
    editingProject = null;
    showProjectModal = true;
  }

  function openEditProject(e) {
    editingProject = e.detail;
    showProjectModal = true;
  }

  async function handleSaveProject(e) {
    const data = e.detail;
    if (data.id) {
      await updateProject(data.id, data);
    } else {
      await createProject(data);
    }
    showProjectModal = false;
    await loadData();
  }

  async function handleDeleteProject(e) {
    await deleteProject(e.detail.id);
    showProjectModal = false;
    await loadData();
  }

</script>

<div class="app">
  <header>
    <div class="header-left">
      {#if !$sidebarOpen}
        <button class="btn-sidebar" on:click={() => ($sidebarOpen = true)} title="Show projects">
          &#9776;
        </button>
      {/if}
      <h1>Uni Project Tracker</h1>
    </div>
    <div class="header-right">
      <Filters />
      <button class="btn-add" on:click={openNewTask}>+ New Task</button>
    </div>
  </header>

  <div class="content">
    <Sidebar on:newProject={openNewProject} on:editProject={openEditProject} />

    <main class="board">
      {#each columns as col}
        <Column
          status={col.status}
          title={col.title}
          on:edit={openEditTask}
          on:drop={handleDrop}
        />
      {/each}
    </main>
  </div>
</div>

{#if showTaskModal}
  <Modal
    task={editingTask}
    on:save={handleSaveTask}
    on:delete={handleDeleteTask}
    on:close={() => (showTaskModal = false)}
  />
{/if}

{#if showProjectModal}
  <ProjectModal
    project={editingProject}
    on:save={handleSaveProject}
    on:delete={handleDeleteProject}
    on:close={() => (showProjectModal = false)}
  />
{/if}

<style>
  .app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1.5rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
    gap: 0.75rem;
    z-index: 10;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  h1 {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 800;
    color: var(--text);
  }

  .btn-sidebar {
    background: none;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    transition: all 0.15s;
  }

  .btn-sidebar:hover {
    background: var(--muted);
    color: var(--text);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .btn-add {
    padding: 0.5rem 1.25rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: opacity 0.15s;
  }

  .btn-add:hover {
    opacity: 0.9;
  }

  .content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .board {
    display: flex;
    gap: 1rem;
    padding: 1.25rem;
    flex: 1;
    overflow-x: auto;
  }
</style>
