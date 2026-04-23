<script>
  import { onMount } from 'svelte';
  import { createEventDispatcher } from 'svelte';
  import { projects, allFiles, coursesDetailed, subsections } from '../stores.js';
  import {
    fetchAllFiles,
    fetchCoursesDetailed,
    fetchSubsections,
    createSubsection,
    deleteSubsection,
    reorderSubsections,
    reorderProjectFiles,
    updateCourseExamDeadline,
    updateProjectFile,
    deleteProjectFile,
    createProjectFile,
    uploadProjectFile,
    fileToBase64,
  } from '../api.js';

  const dispatch = createEventDispatcher();

  let search = '';
  let filterCourse = '';
  let editingCourse = null;
  let editingDeadline = '';
  let addingSubFor = null; // category key currently showing the add-sub input
  let newSubName = '';
  let collapsed = {}; // `${category}|${sub}` → bool

  let showAddFile = false;
  let addingFile = false;
  let newFileProjectId = '';
  let newFileType = 'link';
  let newFileName = '';
  let newFileUrl = '';
  let newFileBlob = null;
  let newFileCategory = 'other';
  let newFileSubsection = '';

  const CATEGORIES = [
    { key: 'course',     title: 'Course material', icon: '\u{1F4DA}' },
    { key: 'assignment', title: 'Assignments',     icon: '\u{1F4DD}' },
    { key: 'lab',        title: 'Labs',            icon: '\u{1F9EA}' },
    { key: 'exam',       title: 'Exam prep',       icon: '\u{1F4CB}' },
    { key: 'other',      title: 'Other',           icon: '\u{1F4C1}' },
  ];

  onMount(loadData);

  async function loadData() {
    $allFiles = await fetchAllFiles();
    $coursesDetailed = await fetchCoursesDetailed();
    $subsections = await fetchSubsections();
  }

  function subsForCategory(catKey) {
    return subsByCategory[catKey] || [];
  }

  async function handleAddSub(catKey) {
    const name = newSubName.trim();
    if (!name) return;
    await createSubsection(catKey, name);
    newSubName = '';
    addingSubFor = null;
    await loadData();
  }

  async function handleDeleteSub(catKey, subName) {
    if (!confirm(`Remove subsection "${subName}"? Files inside will become unfiled.`)) return;
    await deleteSubsection(catKey, subName);
    await loadData();
  }

  async function moveFileToSub(file, subName) {
    await updateProjectFile(file.id, { subsection: subName || null });
    await loadData();
  }

  // Swap two files in the group (same category + same subsection) by delta (-1/+1).
  async function moveFile(file, delta) {
    const list = $allFiles.slice();
    const same = (x) =>
      (x.category || 'other') === (file.category || 'other') &&
      (x.subsection || '') === (file.subsection || '');
    const groupIds = list.filter(same).map((x) => x.id);
    const pos = groupIds.indexOf(file.id);
    const newPos = pos + delta;
    if (pos < 0 || newPos < 0 || newPos >= groupIds.length) return;

    const neighborId = groupIds[newPos];
    const aI = list.findIndex((x) => x.id === file.id);
    const bI = list.findIndex((x) => x.id === neighborId);
    [list[aI], list[bI]] = [list[bI], list[aI]];
    $allFiles = [...list];
    await reorderProjectFiles(list.map((x) => x.id));
  }

  // Swap two subsections within a category by delta (-1/+1).
  async function moveSub(catKey, subName, delta) {
    const current = subsForCategory(catKey);
    const pos = current.indexOf(subName);
    const newPos = pos + delta;
    if (pos < 0 || newPos < 0 || newPos >= current.length) return;
    const next = current.slice();
    [next[pos], next[newPos]] = [next[newPos], next[pos]];
    const others = $subsections.filter((s) => s.category !== catKey);
    const reordered = next.map((name, i) => ({
      category: catKey,
      name,
      order_index: i,
      created_at: ($subsections.find((s) => s.category === catKey && s.name === name) || {}).created_at || '',
    }));
    $subsections = [...others, ...reordered];
    await reorderSubsections(catKey, next);
  }

  function toggleCollapse(catKey, subName) {
    const k = `${catKey}|${subName}`;
    collapsed = { ...collapsed, [k]: !collapsed[k] };
  }

  function isCollapsed(catKey, subName) {
    return !!collapsed[`${catKey}|${subName}`];
  }

  $: projectsByCourse = (() => {
    const map = {};
    for (const p of $projects) {
      if (!map[p.course]) map[p.course] = [];
      map[p.course].push(p);
    }
    return map;
  })();

  $: filteredFiles = (() => {
    const q = search.trim().toLowerCase();
    return $allFiles.filter((f) => {
      if (filterCourse && (f.course || '') !== filterCourse) return false;
      if (!q) return true;
      return (
        f.name.toLowerCase().includes(q) ||
        (f.project_name || '').toLowerCase().includes(q) ||
        (f.course || '').toLowerCase().includes(q) ||
        (f.category || '').toLowerCase().includes(q) ||
        (f.file_type || '').toLowerCase().includes(q)
      );
    });
  })();

  // Reactive index keyed by category: { files, groups: [ [subName, files[]] ] }
  // Built off filteredFiles + $subsections so Svelte re-renders when either changes.
  $: byCategory = (() => {
    const result = {};
    const subsByCat = {};
    for (const s of $subsections) {
      (subsByCat[s.category] ||= []).push(s.name);
    }
    for (const cat of CATEGORIES) {
      const catFiles = filteredFiles.filter((f) => (f.category || 'other') === cat.key);
      const declared = subsByCat[cat.key] || [];
      const groups = new Map();
      groups.set('', []);
      for (const sub of declared) groups.set(sub, []);
      for (const f of catFiles) {
        const key = f.subsection || '';
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key).push(f);
      }
      result[cat.key] = { files: catFiles, groups: [...groups.entries()] };
    }
    return result;
  })();

  $: subsByCategory = (() => {
    const out = {};
    for (const cat of CATEGORIES) out[cat.key] = [];
    for (const s of $subsections) (out[s.category] ||= []).push(s.name);
    return out;
  })();

  function daysUntil(dateStr) {
    if (!dateStr) return null;
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    const dl = new Date(dateStr + 'T00:00:00');
    return Math.ceil((dl - now) / (1000 * 60 * 60 * 24));
  }

  function formatDate(dateStr) {
    return new Date(dateStr + 'T00:00:00').toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    });
  }

  function deadlineClass(days) {
    if (days == null) return '';
    if (days < 0) return 'overdue';
    if (days <= 7) return 'urgent';
    if (days <= 30) return 'soon';
    return 'ok';
  }

  function fileIcon(f) {
    if (f.file_type === 'link') return '\u{1F517}';
    if (f.file_type === 'pdf') return '\u{1F4C4}';
    return '\u{1F4CE}';
  }

  function fileExt(f) {
    const src = f.url || f.name || '';
    const i = src.lastIndexOf('.');
    if (i < 0 || i === src.length - 1) return '';
    const ext = src.slice(i + 1).split(/[?#]/)[0].toLowerCase();
    return /^[a-z0-9]{1,8}$/.test(ext) ? ext : '';
  }

  function startEdit(course) {
    editingCourse = course.name;
    editingDeadline = course.exam_deadline || '';
  }

  async function saveDeadline(course) {
    await updateCourseExamDeadline(course.name, editingDeadline);
    editingCourse = null;
    await loadData();
  }

  function cancelEdit() {
    editingCourse = null;
    editingDeadline = '';
  }

  async function changeCategory(file, category) {
    if (category === file.category) return;
    // Subsections belong to their category, so clear it when moving.
    await updateProjectFile(file.id, { category, subsection: null });
    await loadData();
  }

  async function removeFile(file) {
    if (!confirm(`Remove "${file.name}"?`)) return;
    await deleteProjectFile(file.id);
    await loadData();
  }

  function openProject(p) {
    dispatch('openProject', p);
  }

  function openAddFile(catKey = 'other') {
    newFileProjectId = $projects[0]?.id || '';
    newFileType = 'link';
    newFileName = '';
    newFileUrl = '';
    newFileBlob = null;
    newFileCategory = catKey;
    newFileSubsection = '';
    showAddFile = true;
  }

  function closeAddFile() {
    if (addingFile) return;
    showAddFile = false;
  }

  async function submitAddFile() {
    if (!newFileProjectId) return;
    const sub = newFileSubsection || null;
    addingFile = true;
    try {
      if (newFileType === 'link') {
        const name = newFileName.trim();
        const url = newFileUrl.trim();
        if (!name || !url) return;
        const created = await createProjectFile({
          project_id: newFileProjectId,
          name,
          file_type: 'link',
          url,
          category: newFileCategory,
          subsection: sub,
        });
        if (sub && created?.id) {
          await updateProjectFile(created.id, { subsection: sub });
        }
      } else {
        if (!newFileBlob) return;
        const b64 = await fileToBase64(newFileBlob);
        const displayName = newFileName.trim() || newFileBlob.name;
        const created = await uploadProjectFile(
          newFileProjectId,
          displayName,
          b64,
          newFileBlob.name,
          newFileCategory,
          sub,
        );
        if (sub && created?.id) {
          await updateProjectFile(created.id, { subsection: sub });
        }
      }
      showAddFile = false;
      await loadData();
    } finally {
      addingFile = false;
    }
  }

  function handleAddFileKeydown(e) {
    if (e.key === 'Escape' && showAddFile) closeAddFile();
  }

  $: subsForNewFile = subsForCategory(newFileCategory);
  $: if (showAddFile && newFileSubsection && !subsForNewFile.includes(newFileSubsection)) {
    newFileSubsection = '';
  }
</script>

<svelte:window on:keydown={handleAddFileKeydown} />

<div class="courses-page">
  <main class="library">
    <div class="library-header">
      <h2>Course Library</h2>
      <select class="course-filter" bind:value={filterCourse} title="Filter by course">
        <option value="">All courses</option>
        {#each $coursesDetailed as c (c.name)}
          <option value={c.name}>{c.name}</option>
        {/each}
      </select>
      <input
        type="search"
        class="search"
        placeholder="Search files, links, projects..."
        bind:value={search}
      />
      {#if filterCourse || search}
        <button class="clear-btn" on:click={() => { filterCourse = ''; search = ''; }}>Clear</button>
      {/if}
      <button class="add-file-btn" on:click={() => openAddFile()} disabled={$projects.length === 0} title={$projects.length === 0 ? 'Create a project first' : 'Add file'}>
        + Add file
      </button>
    </div>

    <div class="columns">
      {#each CATEGORIES as cat}
        {@const cell = byCategory[cat.key] || { files: [], groups: [] }}
        {@const files = cell.files}
        {@const groups = cell.groups}
        <section class="column">
          <header class="column-header">
            <span class="column-title">
              <span class="icon">{cat.icon}</span>{cat.title}
            </span>
            <span class="count">{files.length}</span>
          </header>

          <div class="sub-add-row">
            {#if addingSubFor === cat.key}
              <input
                class="sub-add-input"
                type="text"
                placeholder="lesson_01"
                bind:value={newSubName}
                on:keydown={(e) => {
                  if (e.key === 'Enter') handleAddSub(cat.key);
                  if (e.key === 'Escape') { addingSubFor = null; newSubName = ''; }
                }}
              />
              <button class="btn-mini save" on:click={() => handleAddSub(cat.key)}>Add</button>
              <button class="btn-mini" on:click={() => { addingSubFor = null; newSubName = ''; }}>&#x2715;</button>
            {:else}
              <button class="btn-add-sub" on:click={() => { addingSubFor = cat.key; newSubName = ''; }}>
                + Subsection
              </button>
            {/if}
          </div>

          <div class="column-body">
            {#each groups as [subName, subFiles] (subName)}
              {#if subName !== '' || subFiles.length > 0}
                <div class="sub-group">
                  <!-- svelte-ignore a11y-click-events-have-key-events -->
                  <!-- svelte-ignore a11y-no-static-element-interactions -->
                  <div class="sub-header" on:click={() => toggleCollapse(cat.key, subName)}>
                    <span class="sub-caret">{isCollapsed(cat.key, subName) ? '\u25B8' : '\u25BE'}</span>
                    <span class="sub-name">{subName || 'Unfiled'}</span>
                    <span class="sub-count">{subFiles.length}</span>
                    {#if subName}
                      <button
                        class="btn-move"
                        on:click|stopPropagation={() => moveSub(cat.key, subName, -1)}
                        title="Move up"
                      >&#9650;</button>
                      <button
                        class="btn-move"
                        on:click|stopPropagation={() => moveSub(cat.key, subName, 1)}
                        title="Move down"
                      >&#9660;</button>
                      <button
                        class="btn-sub-del"
                        on:click|stopPropagation={() => handleDeleteSub(cat.key, subName)}
                        title="Delete subsection"
                      >&#x2715;</button>
                    {/if}
                  </div>

                  {#if !isCollapsed(cat.key, subName)}
                    <div class="sub-body">
                      {#each subFiles as f (f.id)}
                        <div class="file-card">
                          <div class="file-top">
                            <span class="file-icon">{fileIcon(f)}</span>
                            <a href={f.url} target="_blank" rel="noopener noreferrer" class="file-name" title={f.name}>
                              {f.name}
                            </a>
                            {#if fileExt(f)}<span class="file-ext">.{fileExt(f)}</span>{/if}
                          </div>
                          <div class="file-meta">
                            <span class="chip course">{f.course}</span>
                            <span class="chip project" title="Open project" on:click={() => openProject({ id: f.project_id })}
                                  on:keydown={(e) => e.key === 'Enter' && openProject({ id: f.project_id })}
                                  role="button" tabindex="0">
                              {f.project_name}
                            </span>
                          </div>
                          <div class="file-actions">
                            <select
                              class="cat-select"
                              value={f.category || 'other'}
                              on:change={(e) => changeCategory(f, e.target.value)}
                              title="Change category"
                            >
                              {#each CATEGORIES as c}
                                <option value={c.key}>{c.title}</option>
                              {/each}
                            </select>
                            <select
                              class="cat-select"
                              value={f.subsection || ''}
                              on:change={(e) => moveFileToSub(f, e.target.value)}
                              title="Move to subsection"
                            >
                              <option value="">— Unfiled —</option>
                              {#each subsForCategory(f.category || 'other') as s}
                                <option value={s}>{s}</option>
                              {/each}
                            </select>
                          </div>
                          <div class="file-btns">
                            <button class="btn-move" on:click={() => moveFile(f, -1)} title="Move up">&#9650;</button>
                            <button class="btn-move" on:click={() => moveFile(f, 1)} title="Move down">&#9660;</button>
                            <button class="btn-remove" on:click={() => removeFile(f)} title="Remove">&#x2715;</button>
                          </div>
                        </div>
                      {:else}
                        <p class="empty-sub">Drop files here by changing their subsection.</p>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
            {/each}

            {#if files.length === 0 && subsForCategory(cat.key).length === 0}
              <p class="empty-col">{search ? 'No matches' : 'Empty'}</p>
            {/if}
          </div>
        </section>
      {/each}
    </div>
  </main>

  <aside class="courses-panel">
    <div class="panel-header">
      <h3>Courses</h3>
      <span class="panel-sub">{$coursesDetailed.length} total</span>
    </div>

    <div class="course-list">
      {#each $coursesDetailed as c (c.name)}
        {@const d = daysUntil(c.exam_deadline)}
        {@const projList = projectsByCourse[c.name] || []}
        <div class="course-card">
          <div class="course-top">
            <h4>{c.name}</h4>
            <button class="btn-edit" on:click={() => startEdit(c)} title="Set exam deadline">&#9998;</button>
          </div>

          {#if editingCourse === c.name}
            <div class="deadline-edit">
              <input type="date" bind:value={editingDeadline} />
              <div class="edit-actions">
                <button class="btn-mini save" on:click={() => saveDeadline(c)}>Save</button>
                <button class="btn-mini" on:click={cancelEdit}>Cancel</button>
              </div>
            </div>
          {:else if c.exam_deadline}
            <div class="exam-row">
              <span class="exam-label {deadlineClass(d)}">
                Exam
                {#if d < 0}
                  {Math.abs(d)}d ago
                {:else if d === 0}
                  today
                {:else}
                  in {d}d
                {/if}
              </span>
              <span class="exam-date">{formatDate(c.exam_deadline)}</span>
            </div>
          {:else}
            <p class="no-exam">No exam date set</p>
          {/if}

          <div class="projects-sublist">
            <span class="sub-label">Projects &amp; deadlines</span>
            {#each projList as p (p.id)}
              {@const pd = daysUntil(p.deadline)}
              <!-- svelte-ignore a11y-click-events-have-key-events -->
              <!-- svelte-ignore a11y-no-static-element-interactions -->
              <div class="proj-row" on:click={() => openProject(p)} role="button" tabindex="0">
                <span class="proj-name">{p.name}</span>
                <span class="proj-deadline {deadlineClass(pd)}">
                  {#if pd < 0}{Math.abs(pd)}d late
                  {:else if pd === 0}today
                  {:else}{pd}d{/if}
                </span>
              </div>
            {:else}
              <p class="no-projects">No projects</p>
            {/each}
          </div>
        </div>
      {:else}
        <p class="empty">No courses yet. Create a project to get started.</p>
      {/each}
    </div>
  </aside>
</div>

{#if showAddFile}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="overlay" on:click|self={closeAddFile}>
    <div class="add-modal" role="dialog" aria-modal="true">
      <div class="add-modal-header">
        <h3>Add file to library</h3>
        <button class="btn-close" on:click={closeAddFile} title="Close">&#x2715;</button>
      </div>

      <form class="add-modal-body" on:submit|preventDefault={submitAddFile}>
        <label>
          <span>Project <em>*</em></span>
          <select bind:value={newFileProjectId} required>
            <option value="" disabled>Select a project</option>
            {#each $projects as p (p.id)}
              <option value={p.id}>{p.name} ({p.course})</option>
            {/each}
          </select>
        </label>

        <div class="type-row">
          <label class="type-opt" class:active={newFileType === 'link'}>
            <input type="radio" bind:group={newFileType} value="link" />
            <span>&#128279; Link</span>
          </label>
          <label class="type-opt" class:active={newFileType === 'pdf'}>
            <input type="radio" bind:group={newFileType} value="pdf" />
            <span>&#128196; PDF</span>
          </label>
          <label class="type-opt" class:active={newFileType === 'file'}>
            <input type="radio" bind:group={newFileType} value="file" />
            <span>&#128206; File</span>
          </label>
        </div>

        <label>
          <span>{newFileType === 'link' ? 'Name' : 'Display name'} {#if newFileType === 'link'}<em>*</em>{/if}</span>
          <input
            type="text"
            bind:value={newFileName}
            placeholder={newFileType === 'link' ? 'Link name' : 'Optional display name'}
            required={newFileType === 'link'}
          />
        </label>

        {#if newFileType === 'link'}
          <label>
            <span>URL <em>*</em></span>
            <input type="url" bind:value={newFileUrl} placeholder="https://..." required />
          </label>
        {:else}
          <label>
            <span>File <em>*</em></span>
            <input
              type="file"
              accept={newFileType === 'pdf' ? '.pdf' : undefined}
              on:change={(e) => (newFileBlob = e.target.files[0] || null)}
              required
            />
          </label>
        {/if}

        <div class="row">
          <label class="flex-1">
            <span>Category</span>
            <select bind:value={newFileCategory}>
              {#each CATEGORIES as c}
                <option value={c.key}>{c.title}</option>
              {/each}
            </select>
          </label>
          <label class="flex-1">
            <span>Subsection</span>
            <select bind:value={newFileSubsection}>
              <option value="">— Unfiled —</option>
              {#each subsForNewFile as s}
                <option value={s}>{s}</option>
              {/each}
            </select>
          </label>
        </div>

        <div class="add-modal-actions">
          <button type="button" class="btn-cancel" on:click={closeAddFile} disabled={addingFile}>Cancel</button>
          <button type="submit" class="btn-save" disabled={addingFile || !newFileProjectId}>
            {addingFile ? 'Adding…' : 'Add file'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .courses-page {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .library {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 1rem 1.25rem;
    overflow: hidden;
  }

  .library-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .library-header h2 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 700;
    color: var(--text);
  }

  .search {
    flex: 1;
    max-width: 420px;
    padding: 0.55rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.88rem;
  }

  .search:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  .course-filter {
    padding: 0.55rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.88rem;
    cursor: pointer;
    max-width: 220px;
  }

  .course-filter:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  .clear-btn {
    padding: 0.55rem 0.9rem;
    border: none;
    border-radius: 8px;
    background: var(--muted);
    color: var(--text);
    font-size: 0.82rem;
    cursor: pointer;
  }

  .clear-btn:hover { background: var(--border); }

  .add-file-btn {
    padding: 0.55rem 0.95rem;
    border: none;
    border-radius: 8px;
    background: var(--accent);
    color: white;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
  }

  .add-file-btn:hover:not(:disabled) { opacity: 0.9; }
  .add-file-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .columns {
    display: flex;
    gap: 0.85rem;
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
    min-height: 0;
  }

  .column {
    flex: 1 0 260px;
    max-width: 320px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .column-header {
    padding: 0.65rem 0.85rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .column-title {
    font-weight: 700;
    font-size: 0.85rem;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .icon { font-size: 1rem; }

  .count {
    font-size: 0.7rem;
    color: var(--text-secondary);
    background: var(--muted);
    border-radius: 10px;
    padding: 0.1rem 0.45rem;
    font-weight: 600;
  }

  .sub-add-row {
    padding: 0.45rem 0.6rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .btn-add-sub {
    background: none;
    border: 1px dashed var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.72rem;
    font-weight: 600;
    padding: 0.3rem 0.5rem;
    border-radius: 6px;
    width: 100%;
  }

  .btn-add-sub:hover {
    background: var(--muted);
    color: var(--accent);
    border-color: var(--accent);
    border-style: solid;
  }

  .sub-add-input {
    flex: 1;
    padding: 0.3rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.78rem;
  }

  .sub-add-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .column-body {
    flex: 1;
    overflow-y: auto;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .sub-group {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }

  .sub-header {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.4rem 0.55rem;
    background: var(--muted);
    cursor: pointer;
    user-select: none;
  }

  .sub-caret {
    font-size: 0.7rem;
    color: var(--text-secondary);
    width: 0.8rem;
  }

  .sub-name {
    flex: 1;
    font-size: 0.78rem;
    font-weight: 700;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub-count {
    font-size: 0.65rem;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.05rem 0.4rem;
  }

  .btn-move {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.55rem;
    padding: 0.12rem 0.32rem;
    border-radius: 4px;
    opacity: 0.7;
    line-height: 1;
  }

  .btn-move:hover {
    opacity: 1;
    color: var(--accent);
    border-color: var(--accent);
  }

  .btn-sub-del {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.75rem;
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
    opacity: 0.5;
  }

  .btn-sub-del:hover {
    opacity: 1;
    color: #dc2626;
    background: #fef2f2;
  }

  .sub-body {
    padding: 0.45rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    background: var(--surface);
  }

  .empty-sub {
    margin: 0;
    padding: 0.5rem 0.25rem;
    font-size: 0.72rem;
    color: var(--text-secondary);
    font-style: italic;
    text-align: center;
  }

  .file-card {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.55rem 0.65rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .file-card:hover { background: var(--muted); }

  .file-top {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    min-width: 0;
  }

  .file-icon { flex-shrink: 0; }

  .file-name {
    flex: 1;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--accent);
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-name:hover { text-decoration: underline; }

  .file-ext {
    flex-shrink: 0;
    font-size: 0.65rem;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--muted);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.05rem 0.35rem;
    text-transform: uppercase;
  }

  .file-meta {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .chip {
    font-size: 0.68rem;
    padding: 0.12rem 0.45rem;
    border-radius: 10px;
    background: var(--muted);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .chip.course {
    background: rgba(99, 102, 241, 0.08);
    color: var(--accent);
    border-color: rgba(99, 102, 241, 0.25);
    font-weight: 600;
  }

  .chip.project { cursor: pointer; }
  .chip.project:hover { background: var(--accent); color: white; }

  .file-actions {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .file-btns {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 0.25rem;
  }

  .cat-select {
    flex: 1;
    min-width: 0;
    padding: 0.25rem 0.35rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.72rem;
  }

  .btn-remove {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.78rem;
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    opacity: 0.5;
  }

  .btn-remove:hover {
    opacity: 1;
    color: #dc2626;
    background: #fef2f2;
  }

  .empty-col {
    color: var(--text-secondary);
    font-size: 0.78rem;
    text-align: center;
    padding: 1rem 0;
    margin: 0;
  }

  .courses-panel {
    width: 320px;
    min-width: 320px;
    background: var(--surface);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
    padding: 1rem 1rem 0.75rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--text);
  }

  .panel-sub {
    font-size: 0.72rem;
    color: var(--text-secondary);
  }

  .course-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .course-card {
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
  }

  .course-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .course-top h4 {
    margin: 0;
    font-size: 0.88rem;
    font-weight: 700;
    color: var(--text);
  }

  .btn-edit {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.85rem;
    opacity: 0.5;
  }

  .btn-edit:hover { opacity: 1; color: var(--accent); }

  .exam-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    margin-bottom: 0.6rem;
  }

  .exam-label {
    font-weight: 600;
    padding: 0.1rem 0.45rem;
    border-radius: 4px;
  }

  .exam-label.overdue { color: #dc2626; background: #fef2f2; }
  .exam-label.urgent  { color: #ea580c; background: #fff7ed; }
  .exam-label.soon    { color: #ca8a04; background: #fefce8; }
  .exam-label.ok      { color: #16a34a; background: #f0fdf4; }

  .exam-date { color: var(--text-secondary); }

  .no-exam {
    margin: 0 0 0.6rem;
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-style: italic;
  }

  .deadline-edit {
    margin-bottom: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .deadline-edit input {
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.8rem;
  }

  .edit-actions { display: flex; gap: 0.35rem; justify-content: flex-end; }

  .btn-mini {
    padding: 0.25rem 0.6rem;
    background: var(--muted);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 0.72rem;
    border-radius: 5px;
    cursor: pointer;
  }

  .btn-mini.save {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .projects-sublist {
    border-top: 1px dashed var(--border);
    padding-top: 0.5rem;
  }

  .sub-label {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    font-weight: 600;
    display: block;
    margin-bottom: 0.35rem;
  }

  .proj-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0.5rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.78rem;
  }

  .proj-row:hover { background: var(--muted); }

  .proj-name {
    color: var(--text);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    margin-right: 0.5rem;
  }

  .proj-deadline {
    font-size: 0.7rem;
    font-weight: 600;
    padding: 0.08rem 0.4rem;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .proj-deadline.overdue { color: #dc2626; background: #fef2f2; }
  .proj-deadline.urgent  { color: #ea580c; background: #fff7ed; }
  .proj-deadline.soon    { color: #ca8a04; background: #fefce8; }
  .proj-deadline.ok      { color: #16a34a; background: #f0fdf4; }

  .no-projects, .empty {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-style: italic;
    padding: 0.4rem 0;
  }

  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1rem;
  }

  .add-modal {
    background: var(--surface);
    border-radius: 14px;
    width: 100%;
    max-width: 460px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
    overflow: hidden;
  }

  .add-modal-header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .add-modal-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--text);
  }

  .btn-close {
    background: none;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    width: 26px;
    height: 26px;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .btn-close:hover { background: var(--muted); color: var(--text); }

  .add-modal-body {
    padding: 1rem 1.25rem 1.25rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .add-modal-body label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.78rem;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .add-modal-body label em { color: #ef4444; font-style: normal; }

  .add-modal-body input[type='text'],
  .add-modal-body input[type='url'],
  .add-modal-body input[type='file'],
  .add-modal-body select {
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.85rem;
    font-family: inherit;
  }

  .add-modal-body input:focus,
  .add-modal-body select:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  .type-row {
    display: flex;
    gap: 0.4rem;
  }

  .type-opt {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 0.45rem 0.5rem;
    background: var(--bg);
    color: var(--text-secondary);
    cursor: pointer;
    text-align: center;
    font-size: 0.78rem;
    font-weight: 600;
  }

  .type-opt input { display: none; }

  .type-opt.active {
    background: rgba(99, 102, 241, 0.12);
    color: var(--accent);
    border-color: var(--accent);
  }

  .row { display: flex; gap: 0.6rem; }
  .flex-1 { flex: 1; min-width: 0; }

  .add-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.5rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border);
  }

  .btn-cancel,
  .btn-save {
    padding: 0.5rem 1.1rem;
    border: none;
    border-radius: 8px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-cancel { background: var(--muted); color: var(--text); }
  .btn-cancel:hover:not(:disabled) { background: var(--border); }
  .btn-save { background: var(--accent); color: white; }
  .btn-save:hover:not(:disabled) { opacity: 0.9; }
  .btn-save:disabled,
  .btn-cancel:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
