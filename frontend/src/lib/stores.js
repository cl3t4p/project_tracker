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

// Current view: 'board', 'courses', or { page: 'project', projectId: '...' }
export const currentView = writable('board');

// All project files with context (for the courses/library page)
export const allFiles = writable([]);
// Courses with exam deadline info (for the courses page)
export const coursesDetailed = writable([]);
// User-defined subsections (per category)
export const subsections = writable([]);

// Project files (loaded when viewing a project detail page)
export const projectFiles = writable([]);

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

function compareTasks(a, b) {
  const FAR = '9999-12-31';
  const ad = a.due_date || FAR;
  const bd = b.due_date || FAR;
  if (ad !== bd) return ad < bd ? -1 : 1;
  const ao = a.order_index ?? 0;
  const bo = b.order_index ?? 0;
  if (ao !== bo) return ao - bo;
  return (a.created_at || '').localeCompare(b.created_at || '');
}

export function tasksByStatus(status) {
  return derived(filteredTasks, ($ft) =>
    $ft.filter((t) => t.status === status).slice().sort(compareTasks)
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
