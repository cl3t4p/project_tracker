import { writable, derived } from 'svelte/store';

export const projects = writable([]);
export const tasks = writable([]);
export const courses = writable([]);

// Filters
export const filterProject = writable('');
export const filterPriority = writable('');
export const filterSearch = writable('');

// Sidebar
export const sidebarOpen = writable(true);

// Filtered tasks for the kanban
export const filteredTasks = derived(
  [tasks, filterProject, filterPriority, filterSearch],
  ([$tasks, $filterProject, $filterPriority, $filterSearch]) => {
    return $tasks.filter((t) => {
      if ($filterProject && t.project_id !== $filterProject) return false;
      if ($filterPriority && t.priority !== $filterPriority) return false;
      if ($filterSearch) {
        const q = $filterSearch.toLowerCase();
        if (
          !t.name.toLowerCase().includes(q) &&
          !t.description.toLowerCase().includes(q)
        )
          return false;
      }
      return true;
    });
  }
);

export function tasksByStatus(status) {
  return derived(filteredTasks, ($ft) =>
    $ft.filter((t) => t.status === status)
  );
}

// Tasks grouped by project (for sidebar)
export const tasksByProject = derived(
  [projects, tasks],
  ([$projects, $tasks]) => {
    return $projects.map((p) => ({
      ...p,
      tasks: $tasks.filter((t) => t.project_id === p.id),
    }));
  }
);
