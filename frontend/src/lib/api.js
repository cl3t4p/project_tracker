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

// ── AI ──

export async function aiGenerateTasks(
  projectId,
  { count = 5, pdfBase64 = null, startDate = '', deadline = '' } = {},
) {
  const res = await fetch(`${BASE}/projects/${projectId}/ai/generate-tasks`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      count,
      pdf_base64: pdfBase64,
      start_date: startDate,
      deadline,
    }),
  });
  if (!res.ok) throw new Error((await res.json()).error || 'AI request failed');
  return res.json();
}

export async function aiStatus() {
  try {
    const res = await fetch(`${BASE}/ai/status`);
    if (!res.ok) return { configured: false };
    return res.json();
  } catch {
    return { configured: false };
  }
}

export async function aiProjectFromPdf(pdfBase64, count = 6) {
  const res = await fetch(`${BASE}/ai/project-from-pdf`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pdf_base64: pdfBase64, count }),
  });
  if (!res.ok) throw new Error((await res.json()).error || 'AI request failed');
  return res.json();
}

export async function createTasksBulk(projectId, tasks) {
  const res = await fetch(`${BASE}/tasks/bulk`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ project_id: projectId, tasks }),
  });
  if (!res.ok) throw new Error('Bulk create failed');
  return res.json();
}

export async function aiManualPromptTasks(
  projectId,
  { count = 5, startDate = '', deadline = '' } = {},
) {
  const res = await fetch(`${BASE}/projects/${projectId}/ai/manual-prompt`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ count, start_date: startDate, deadline }),
  });
  if (!res.ok) throw new Error('Failed to build prompt');
  return res.json();
}

export async function aiManualPromptProject(count = 6) {
  const res = await fetch(`${BASE}/ai/manual-prompt/project`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ count }),
  });
  if (!res.ok) throw new Error('Failed to build prompt');
  return res.json();
}

export function fileToBase64(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result; // "data:...;base64,XXXX"
      const comma = result.indexOf(',');
      resolve(comma >= 0 ? result.slice(comma + 1) : result);
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}
