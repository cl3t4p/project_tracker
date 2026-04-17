const BASE = '/api';

// ── Projects ──

export async function fetchProjects() {
  const res = await fetch(`${BASE}/projects`);
  return res.json();
}

export async function createProject(project) {
  const res = await fetch(`${BASE}/projects`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(project),
  });
  return res.json();
}

export async function updateProject(id, updates) {
  const res = await fetch(`${BASE}/projects/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(updates),
  });
  return res.json();
}

export async function deleteProject(id) {
  await fetch(`${BASE}/projects/${id}`, { method: 'DELETE' });
}

// ── Tasks ──

export async function fetchTasks() {
  const res = await fetch(`${BASE}/tasks`);
  return res.json();
}

export async function createTask(task) {
  const res = await fetch(`${BASE}/tasks`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(task),
  });
  return res.json();
}

export async function updateTask(id, updates) {
  const res = await fetch(`${BASE}/tasks/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(updates),
  });
  return res.json();
}

export async function deleteTask(id) {
  await fetch(`${BASE}/tasks/${id}`, { method: 'DELETE' });
}

// ── Courses ──

export async function fetchCourses() {
  const res = await fetch(`${BASE}/courses`);
  return res.json();
}
