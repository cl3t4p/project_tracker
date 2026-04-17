<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import {
    aiProjectFromPdf,
    aiManualPromptProject,
    aiStatus,
    createProject,
    createTasksBulk,
    fileToBase64,
  } from '../api.js';

  const dispatch = createEventDispatcher();

  let apiConfigured = false;
  let mode = 'manual'; // 'api' | 'manual'
  let pdfFile = null;
  let count = 6;
  let loading = false;
  let error = '';
  let draft = null;

  onMount(async () => {
    const { configured } = await aiStatus();
    apiConfigured = configured;
    mode = configured ? 'api' : 'manual';
  });
  let manualPrompt = '';
  let manualResponse = '';
  let copied = false;
  const RESPONSE_PLACEHOLDER = '{"name": "...", "course": "...", "tasks": [...]}';
  // draft = { name, course, description, deadline, tasks: [{...,_selected}] }

  function onPdfChange(e) {
    pdfFile = e.target.files?.[0] || null;
  }

  async function handleAnalyze() {
    if (!pdfFile || loading) return;
    loading = true;
    error = '';
    try {
      const b64 = await fileToBase64(pdfFile);
      const res = await aiProjectFromPdf(b64, count);
      draft = {
        name: res.name || '',
        course: res.course || '',
        description: res.description || '',
        deadline: res.deadline || '',
        tasks: (res.tasks || []).map((t) => ({
          ...t,
          due_date: t.due_date || '',
          _selected: true,
        })),
      };
    } catch (e) {
      error = e.message || 'Failed to analyze PDF';
    } finally {
      loading = false;
    }
  }

  async function handleCreate() {
    if (!draft) return;
    if (!draft.name.trim() || !draft.course.trim() || !draft.deadline) {
      error = 'Name, course and deadline are required';
      return;
    }
    loading = true;
    error = '';
    try {
      const project = await createProject({
        name: draft.name.trim(),
        course: draft.course.trim(),
        description: draft.description.trim(),
        deadline: draft.deadline,
      });
      const picked = draft.tasks
        .filter((t) => t._selected)
        .map(({ name, description, priority, due_date }) => ({
          name,
          description,
          priority,
          due_date: due_date || null,
        }));
      if (picked.length > 0) {
        await createTasksBulk(project.id, picked);
      }
      dispatch('created');
    } catch (e) {
      error = e.message || 'Create failed';
    } finally {
      loading = false;
    }
  }

  async function handleLoadPrompt() {
    if (loading) return;
    loading = true;
    error = '';
    try {
      const res = await aiManualPromptProject(count);
      manualPrompt = res.prompt;
    } catch (e) {
      error = e.message || 'Failed to load prompt';
    } finally {
      loading = false;
    }
  }

  async function copyPrompt() {
    try {
      await navigator.clipboard.writeText(manualPrompt);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      error = 'Clipboard blocked — select and copy manually';
    }
  }

  function handleImport() {
    error = '';
    let raw = manualResponse.trim();
    const fence = raw.match(/```(?:json)?\s*([\s\S]*?)```/i);
    if (fence) raw = fence[1].trim();
    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch {
      error = 'Response is not valid JSON';
      return;
    }
    if (!parsed || typeof parsed.name !== 'string') {
      error = 'Missing "name" in pasted JSON';
      return;
    }
    const tasks = Array.isArray(parsed.tasks) ? parsed.tasks : [];
    draft = {
      name: parsed.name || '',
      course: parsed.course || 'Unknown',
      description: parsed.description || '',
      deadline: typeof parsed.deadline === 'string' ? parsed.deadline : '',
      tasks: tasks
        .filter((t) => t && typeof t.name === 'string')
        .map((t) => ({
          name: t.name,
          description: t.description || '',
          priority: ['low', 'medium', 'high'].includes(t.priority) ? t.priority : 'medium',
          due_date: typeof t.due_date === 'string' ? t.due_date : '',
          _selected: true,
        })),
    };
  }

  function handleKeydown(e) {
    if (e.key === 'Escape') dispatch('close');
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="overlay" on:click|self={() => dispatch('close')} on:keydown={() => {}}>
  <div class="modal" role="dialog" aria-modal="true">
    <h2>Create project from PDF</h2>

    {#if !draft}
      {#if apiConfigured}
        <div class="tabs">
          <button class:active={mode === 'api'} on:click={() => (mode = 'api')}>API</button>
          <button class:active={mode === 'manual'} on:click={() => (mode = 'manual')}>Manual (copy/paste)</button>
        </div>
      {/if}

      <label>
        <span>Number of initial tasks</span>
        <input type="number" min="1" max="15" bind:value={count} />
      </label>

      {#if mode === 'api'}
        <label>
          <span>Project brief (PDF) <em>*</em></span>
          <input type="file" accept="application/pdf" on:change={onPdfChange} />
          {#if pdfFile}<small class="muted">{pdfFile.name}</small>{/if}
        </label>
      {:else}
        <div class="manual">
          {#if !manualPrompt}
            <p class="hint">Build a prompt you can paste into any chatbot. Attach your PDF brief in that chat directly.</p>
          {:else}
            <label>
              <span>Prompt <button class="btn-copy" on:click={copyPrompt}>{copied ? 'Copied' : 'Copy'}</button></span>
              <textarea readonly rows="8" value={manualPrompt}></textarea>
            </label>
            <label>
              <span>Paste the chatbot's JSON response</span>
              <textarea bind:value={manualResponse} rows="6" placeholder={RESPONSE_PLACEHOLDER}></textarea>
            </label>
          {/if}
        </div>
      {/if}

      {#if error}<p class="error">{error}</p>{/if}

      <div class="actions">
        <button type="button" class="btn-cancel" on:click={() => dispatch('close')}>Cancel</button>
        {#if mode === 'api'}
          <button type="button" class="btn-save" disabled={!pdfFile || loading} on:click={handleAnalyze}>
            {loading ? 'Analyzing…' : 'Analyze PDF'}
          </button>
        {:else if !manualPrompt}
          <button type="button" class="btn-save" disabled={loading} on:click={handleLoadPrompt}>
            {loading ? 'Loading…' : 'Build prompt'}
          </button>
        {:else}
          <button type="button" class="btn-save" disabled={!manualResponse.trim()} on:click={handleImport}>
            Import JSON
          </button>
        {/if}
      </div>
    {:else}
      <label>
        <span>Project name <em>*</em></span>
        <input type="text" bind:value={draft.name} />
      </label>
      <label>
        <span>Course <em>*</em></span>
        <input type="text" bind:value={draft.course} />
      </label>
      <label>
        <span>Description</span>
        <textarea bind:value={draft.description} rows="3"></textarea>
      </label>
      <label>
        <span>Deadline <em>*</em></span>
        <input type="date" bind:value={draft.deadline} />
      </label>

      <p class="hint">Initial tasks — uncheck to skip. Dates are suggested finish targets.</p>
      <div class="suggestions">
        {#each draft.tasks as s, i (i)}
          <div class="suggestion" class:dim={!s._selected}>
            <label class="row">
              <span class="order-num">{i + 1}.</span>
              <input type="checkbox" bind:checked={s._selected} />
              <input class="s-name" type="text" bind:value={s.name} />
              <select bind:value={s.priority}>
                <option value="low">low</option>
                <option value="medium">medium</option>
                <option value="high">high</option>
              </select>
            </label>
            <label class="row">
              <span class="date-label">Finish by</span>
              <input type="date" bind:value={s.due_date} />
            </label>
            <textarea bind:value={s.description} rows="2"></textarea>
          </div>
        {/each}
      </div>

      {#if error}<p class="error">{error}</p>{/if}

      <div class="actions">
        <button type="button" class="btn-cancel" on:click={() => (draft = null)}>Back</button>
        <button type="button" class="btn-save" disabled={loading} on:click={handleCreate}>
          {loading ? 'Creating…' : 'Create project'}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.4);
    backdrop-filter: blur(4px);
    display: flex; align-items: center; justify-content: center;
    z-index: 100; padding: 1rem;
  }
  .modal {
    background: var(--surface);
    border-radius: 16px;
    padding: 1.75rem;
    width: 100%;
    max-width: 620px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0,0,0,0.15);
  }
  h2 { margin: 0 0 1rem; font-size: 1.15rem; color: var(--text); }
  label { display: flex; flex-direction: column; gap: 0.3rem; margin-bottom: 1rem; }
  label span { font-size: 0.8rem; font-weight: 600; color: var(--text-secondary); }
  label em { color: #ef4444; font-style: normal; }
  input[type='number'], input[type='text'], input[type='date'], textarea, select {
    padding: 0.5rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg); color: var(--text);
    font-size: 0.875rem; font-family: inherit;
  }
  textarea { resize: vertical; }
  .hint { font-size: 0.85rem; color: var(--text-secondary); margin: 0 0 0.75rem; }
  .suggestions { display: flex; flex-direction: column; gap: 0.6rem; margin-bottom: 1rem; }
  .suggestion {
    border: 1px solid var(--border);
    border-radius: 10px; padding: 0.6rem 0.75rem;
    display: flex; flex-direction: column; gap: 0.4rem;
  }
  .suggestion.dim { opacity: 0.5; }
  .row { flex-direction: row; align-items: center; gap: 0.5rem; margin: 0; }
  .s-name { flex: 1; }
  .order-num { font-weight: 700; color: var(--text-secondary); min-width: 1.2rem; }
  .date-label { font-size: 0.75rem; color: var(--text-secondary); min-width: 4rem; }
  .muted { color: var(--text-secondary); font-size: 0.75rem; }
  .error { color: #dc2626; font-size: 0.85rem; margin: 0.5rem 0; }
  .actions {
    display: flex; justify-content: flex-end; gap: 0.5rem;
    margin-top: 1rem; padding-top: 1rem; border-top: 1px solid var(--border);
  }
  button {
    padding: 0.55rem 1.25rem;
    border: none; border-radius: 8px;
    font-size: 0.875rem; font-weight: 600;
    cursor: pointer; transition: all 0.15s;
  }
  button:disabled { opacity: 0.6; cursor: not-allowed; }
  .btn-save { background: var(--accent); color: white; }
  .btn-save:hover:not(:disabled) { opacity: 0.9; }
  .btn-cancel { background: var(--muted); color: var(--text); }
  .btn-cancel:hover { background: var(--border); }

  .tabs { display: flex; gap: 0.25rem; margin-bottom: 1rem; border-bottom: 1px solid var(--border); }
  .tabs button {
    background: none; padding: 0.5rem 0.9rem;
    border: none; border-bottom: 2px solid transparent;
    color: var(--text-secondary); border-radius: 0;
    font-weight: 600; font-size: 0.85rem; cursor: pointer;
  }
  .tabs button.active { color: var(--accent); border-bottom-color: var(--accent); }
  .manual { display: flex; flex-direction: column; }
  .btn-copy {
    background: var(--muted); color: var(--text);
    padding: 0.15rem 0.55rem; font-size: 0.7rem; font-weight: 600;
    border: 1px solid var(--border); border-radius: 6px; margin-left: 0.5rem;
  }
</style>
