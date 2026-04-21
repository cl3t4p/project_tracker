const PT = '/api/project_tracker';
const CO = '/api/course';

// ── Projects ──

export async function fetchProjects() {
  const res = await fetch(`${PT}/projects`);
  return res.json();
}

export async function createProject(project) {
  const res = await fetch(`${PT}/projects`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(project),
  });
  return res.json();
}

export async function updateProject(id, updates) {
  const res = await fetch(`${PT}/projects/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(updates),
  });
  return res.json();
}

export async function deleteProject(id) {
  await fetch(`${PT}/projects/${id}`, { method: 'DELETE' });
}

// ── Tasks ──

export async function fetchTasks() {
  const res = await fetch(`${PT}/tasks`);
  return res.json();
}

export async function createTask(task) {
  const res = await fetch(`${PT}/tasks`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(task),
  });
  return res.json();
}

export async function updateTask(id, updates) {
  const res = await fetch(`${PT}/tasks/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(updates),
  });
  return res.json();
}

export async function deleteTask(id) {
  await fetch(`${PT}/tasks/${id}`, { method: 'DELETE' });
}

// ── Courses (name list used by forms) ──

export async function fetchCourses() {
  const res = await fetch(`${PT}/courses`);
  return res.json();
}

// ── Courses (detailed, with exam deadlines) ──

export async function fetchCoursesDetailed() {
  const res = await fetch(`${CO}/list`);
  return res.json();
}

export async function updateCourseExamDeadline(name, examDeadline) {
  const res = await fetch(`${CO}/${encodeURIComponent(name)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ exam_deadline: examDeadline || null }),
  });
  return res.json();
}

export async function fetchAllFiles() {
  const res = await fetch(`${CO}/files`);
  return res.json();
}

export async function fetchSubsections() {
  const res = await fetch(`${CO}/subsections`);
  return res.json();
}

export async function createSubsection(category, name) {
  const res = await fetch(`${CO}/subsections`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ category, name }),
  });
  return res.json();
}

export async function deleteSubsection(category, name) {
  const params = new URLSearchParams({ category, name });
  await fetch(`${CO}/subsections?${params}`, { method: 'DELETE' });
}

export async function reorderSubsections(category, names) {
  await fetch(`${CO}/subsections/order`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ category, names }),
  });
}

export async function reorderProjectFiles(ids) {
  await fetch(`${PT}/project-files/order`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ids }),
  });
}

// ── Project Files ──

export async function fetchProjectFiles(projectId) {
  const res = await fetch(`${PT}/projects/${projectId}/files`);
  return res.json();
}

export async function createProjectFile(file) {
  const res = await fetch(`${PT}/project-files`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(file),
  });
  return res.json();
}

export async function uploadProjectFile(projectId, name, dataBase64, filename = null, category = 'other', subsection = null) {
  const res = await fetch(`${PT}/project-files/upload`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ project_id: projectId, name, data_base64: dataBase64, filename, category, subsection }),
  });
  return res.json();
}

export async function updateProjectFile(id, updates) {
  const res = await fetch(`${PT}/project-files/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(updates),
  });
  return res.json();
}

export async function deleteProjectFile(id) {
  await fetch(`${PT}/project-files/${id}`, { method: 'DELETE' });
}

// ── AI ──

export async function aiGenerateTasks(
  projectId,
  { count = 5, pdfBase64 = null, startDate = '', deadline = '' } = {},
) {
  const res = await fetch(`${PT}/projects/${projectId}/ai/generate-tasks`, {
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
    const res = await fetch(`${PT}/ai/status`);
    if (!res.ok) return { configured: false };
    return res.json();
  } catch {
    return { configured: false };
  }
}

export async function aiProjectFromPdf(pdfBase64, count = 6) {
  const res = await fetch(`${PT}/ai/project-from-pdf`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pdf_base64: pdfBase64, count }),
  });
  if (!res.ok) throw new Error((await res.json()).error || 'AI request failed');
  return res.json();
}

export async function createTasksBulk(projectId, tasks) {
  const res = await fetch(`${PT}/tasks/bulk`, {
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
  const res = await fetch(`${PT}/projects/${projectId}/ai/manual-prompt`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ count, start_date: startDate, deadline }),
  });
  if (!res.ok) throw new Error('Failed to build prompt');
  return res.json();
}

export async function aiManualPromptProject(count = 6) {
  const res = await fetch(`${PT}/ai/manual-prompt/project`, {
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
