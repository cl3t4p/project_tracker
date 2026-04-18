# Project Tracker

A lightweight kanban-style project tracker for coursework. Each project has
deadlines, tasks across a `todo → in-progress → review → done` board, and an
attached library of links and PDFs that individual tasks can reference.

## Features

- **Projects** with course, deadline, and progress bar
- **Kanban board** with drag-and-drop between columns, priority dots, and
  per-task due dates
- **Project file library** — attach links (URLs) and upload PDFs per project
- **Multiple files per task** — each task can reference any number of its
  project's links / PDFs; chips on the card open them directly
- **AI helpers** (optional) — generate tasks for a project, or bootstrap a
  whole project + task list from a syllabus PDF. Works with Claude or OpenAI.
- **Manual-prompt mode** — if no API key is configured, the app will give you
  a pre-built prompt to paste into any chat UI and an importer for the JSON it
  returns.

## Stack

- **Backend:** Rust + Actix-web 4, SQLite (via rusqlite, bundled)
- **Frontend:** Svelte 5 + Vite (no SvelteKit, SPA)
- **Storage:** single SQLite file + a `files/` directory for uploaded PDFs

## Running with Docker (recommended)

```
cp .env.example .env   # create this if you plan to use AI features
docker compose up --build
```

The app is served on `http://localhost:8080`. Data persists in the `app-data`
named volume.

## Running in dev

Backend:

```
cd backend
cargo run
```

Frontend (separate terminal):

```
cd frontend
npm install
npm run dev
```

The dev frontend proxies `/api/*` to the backend on port 8080 (see
`vite.config.js` if you need to adjust). In production (Docker) the backend
serves the built frontend from `./static`.

## Environment variables

All optional unless you want AI features.

| Variable        | Default              | Purpose                                       |
|-----------------|----------------------|-----------------------------------------------|
| `DATABASE_PATH` | `projects.db`        | Path to the SQLite file. PDFs go next to it in `./files/`. |
| `AI_PROVIDER`   | `claude`             | `claude` / `anthropic` or `openai` / `chatgpt` |
| `AI_API_KEY`    | —                    | Required for AI features                      |
| `AI_MODEL`      | provider-dependent   | Override the default model                    |

## API overview

Namespaced under `/api`:

- `GET/POST/PUT/DELETE /projects[/:id]`
- `GET/POST/PUT/DELETE /tasks[/:id]` — each task returns `files: [{ id, name, url, file_type }]`
- `POST /tasks/bulk` — create many tasks for a project at once
- `GET  /projects/:id/files` — list links + PDFs for a project
- `POST /project-files` — add a link
- `POST /project-files/upload` — upload a PDF (base64-encoded)
- `DELETE /project-files/:id`
- `GET  /files/:filename` — serve an uploaded PDF
- `POST /projects/:id/ai/generate-tasks` — AI task generation
- `POST /ai/project-from-pdf` — AI project-from-syllabus
- `POST /projects/:id/ai/manual-prompt`, `POST /ai/manual-prompt/project` —
  prompt builders for manual LLM use
- `GET  /ai/status` — whether AI is configured

Attach files to a task by sending `file_ids: [...]` on create/update — each ID
must belong to the task's project (managed via the Files modal on the project).

## Data model

- `projects(id, name, course, description, deadline, created_at)`
- `tasks(id, project_id, name, description, priority, status, due_date,
  order_index, created_at)`
- `project_files(id, project_id, name, file_type, url, created_at)`
- `task_files(task_id, file_id)` — many-to-many junction

Schema is created on first boot; new columns/tables are added idempotently.
