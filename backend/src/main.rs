use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

mod ai;
use ai::{ExistingTask, ProjectContext};

// ── Models ──

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Project {
    id: String,
    name: String,
    course: String,
    description: String,
    deadline: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: String,
    project_id: String,
    name: String,
    description: String,
    priority: String, // "low", "medium", "high"
    status: String,   // "todo", "in-progress", "review", "done"
    due_date: Option<String>, // YYYY-MM-DD
    order_index: i64,
    created_at: String,
    #[serde(default)]
    files: Vec<TaskFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TaskFile {
    id: String,
    name: String,
    url: String,
    file_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ProjectFile {
    id: String,
    project_id: String,
    name: String,
    file_type: String, // "link" or "pdf"
    url: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateProject {
    name: String,
    course: String,
    description: Option<String>,
    deadline: String,
}

#[derive(Debug, Deserialize)]
struct UpdateProjectBody {
    name: Option<String>,
    course: Option<String>,
    description: Option<String>,
    deadline: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateTask {
    project_id: String,
    name: String,
    description: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    due_date: Option<String>,
    order_index: Option<i64>,
    file_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CreateProjectFile {
    project_id: String,
    name: String,
    file_type: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct UpdateTaskBody {
    project_id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    due_date: Option<String>,
    order_index: Option<i64>,
    file_ids: Option<Vec<String>>,
}

struct AppState {
    db: Mutex<Connection>,
}

// ── Database ──

fn init_db(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            course TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            deadline TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            priority TEXT NOT NULL DEFAULT 'medium',
            status TEXT NOT NULL DEFAULT 'todo',
            due_date TEXT,
            order_index INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            file_id TEXT REFERENCES project_files(id) ON DELETE SET NULL
        );
        CREATE TABLE IF NOT EXISTS project_files (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            file_type TEXT NOT NULL DEFAULT 'link',
            url TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS task_files (
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            file_id TEXT NOT NULL REFERENCES project_files(id) ON DELETE CASCADE,
            PRIMARY KEY (task_id, file_id)
        );
        PRAGMA foreign_keys = ON;",
    )
    .expect("Failed to initialize database");

    ensure_column(conn, "tasks", "due_date", "TEXT");
    ensure_column(conn, "tasks", "order_index", "INTEGER NOT NULL DEFAULT 0");
    ensure_column(conn, "tasks", "file_id", "TEXT");

    // Migrate any legacy single-file attachments into the junction table
    conn.execute(
        "INSERT OR IGNORE INTO task_files (task_id, file_id)
         SELECT id, file_id FROM tasks WHERE file_id IS NOT NULL AND file_id != ''",
        [],
    ).ok();
}

fn ensure_column(conn: &Connection, table: &str, column: &str, type_sql: &str) {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})")).unwrap();
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    if !names.iter().any(|n| n == column) {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {type_sql}"),
            [],
        )
        .expect("Failed to add column");
    }
}

// ── Project handlers ──

async fn get_projects(data: web::Data<AppState>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, course, description, deadline, created_at FROM projects ORDER BY deadline ASC")
        .unwrap();

    let projects: Vec<Project> = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                course: row.get(2)?,
                description: row.get(3)?,
                deadline: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(projects)
}

async fn create_project(data: web::Data<AppState>, body: web::Json<CreateProject>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let project = Project {
        id: Uuid::new_v4().to_string(),
        name: body.name.clone(),
        course: body.course.clone(),
        description: body.description.clone().unwrap_or_default(),
        deadline: body.deadline.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    conn.execute(
        "INSERT INTO projects (id, name, course, description, deadline, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![project.id, project.name, project.course, project.description, project.deadline, project.created_at],
    ).unwrap();

    HttpResponse::Created().json(project)
}

async fn update_project(data: web::Data<AppState>, path: web::Path<String>, body: web::Json<UpdateProjectBody>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();

    let exists: bool = conn
        .query_row("SELECT COUNT(*) FROM projects WHERE id = ?1", rusqlite::params![id], |row| row.get::<_, i64>(0))
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "Project not found"}));
    }

    if let Some(ref v) = body.name {
        conn.execute("UPDATE projects SET name = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ref v) = body.course {
        conn.execute("UPDATE projects SET course = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ref v) = body.description {
        conn.execute("UPDATE projects SET description = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ref v) = body.deadline {
        conn.execute("UPDATE projects SET deadline = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }

    let project = conn
        .query_row(
            "SELECT id, name, course, description, deadline, created_at FROM projects WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok(Project { id: row.get(0)?, name: row.get(1)?, course: row.get(2)?, description: row.get(3)?, deadline: row.get(4)?, created_at: row.get(5)? }),
        )
        .unwrap();

    HttpResponse::Ok().json(project)
}

async fn delete_project(data: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();
    // Enable foreign keys so CASCADE works
    conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
    let rows = conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![id]).unwrap();
    if rows == 0 {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "Project not found"}));
    }
    HttpResponse::Ok().json(serde_json::json!({"deleted": id}))
}

// ── Task handlers ──

async fn get_tasks(data: web::Data<AppState>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, name, description, priority, status, due_date, order_index, created_at
             FROM tasks
             ORDER BY
                CASE WHEN due_date IS NULL OR due_date = '' THEN 1 ELSE 0 END,
                due_date ASC,
                order_index ASC,
                created_at ASC",
        )
        .unwrap();

    let mut tasks: Vec<Task> = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                priority: row.get(4)?,
                status: row.get(5)?,
                due_date: row.get(6)?,
                order_index: row.get(7)?,
                created_at: row.get(8)?,
                files: Vec::new(),
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = conn
        .prepare(
            "SELECT tf.task_id, pf.id, pf.name, pf.url, pf.file_type
             FROM task_files tf
             JOIN project_files pf ON pf.id = tf.file_id
             ORDER BY pf.created_at ASC",
        )
        .unwrap();

    let mut by_task: HashMap<String, Vec<TaskFile>> = HashMap::new();
    for row in stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                TaskFile {
                    id: row.get(1)?,
                    name: row.get(2)?,
                    url: row.get(3)?,
                    file_type: row.get(4)?,
                },
            ))
        })
        .unwrap()
        .filter_map(|r| r.ok())
    {
        by_task.entry(row.0).or_default().push(row.1);
    }

    for task in &mut tasks {
        if let Some(files) = by_task.remove(&task.id) {
            task.files = files;
        }
    }

    HttpResponse::Ok().json(tasks)
}

async fn create_task(data: web::Data<AppState>, body: web::Json<CreateTask>) -> HttpResponse {
    let conn = data.db.lock().unwrap();

    // Verify project exists
    let exists: bool = conn
        .query_row("SELECT COUNT(*) FROM projects WHERE id = ?1", rusqlite::params![body.project_id], |row| row.get::<_, i64>(0))
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Project not found"}));
    }

    let next_order = next_order_index(&conn, &body.project_id);
    let task = Task {
        id: Uuid::new_v4().to_string(),
        project_id: body.project_id.clone(),
        name: body.name.clone(),
        description: body.description.clone().unwrap_or_default(),
        priority: body.priority.clone().unwrap_or_else(|| "medium".to_string()),
        status: body.status.clone().unwrap_or_else(|| "todo".to_string()),
        due_date: body.due_date.clone().filter(|s| !s.is_empty()),
        order_index: body.order_index.unwrap_or(next_order),
        created_at: chrono::Utc::now().to_rfc3339(),
        files: Vec::new(),
    };

    conn.execute(
        "INSERT INTO tasks (id, project_id, name, description, priority, status, due_date, order_index, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            task.id, task.project_id, task.name, task.description, task.priority,
            task.status, task.due_date, task.order_index, task.created_at
        ],
    ).unwrap();

    if let Some(ids) = &body.file_ids {
        for fid in ids.iter().filter(|s| !s.is_empty()) {
            conn.execute(
                "INSERT OR IGNORE INTO task_files (task_id, file_id) VALUES (?1, ?2)",
                rusqlite::params![task.id, fid],
            ).ok();
        }
    }

    HttpResponse::Created().json(task)
}

fn next_order_index(conn: &Connection, project_id: &str) -> i64 {
    conn.query_row(
        "SELECT COALESCE(MAX(order_index), -1) + 1 FROM tasks WHERE project_id = ?1",
        rusqlite::params![project_id],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
}

async fn update_task(data: web::Data<AppState>, path: web::Path<String>, body: web::Json<UpdateTaskBody>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();

    let exists: bool = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE id = ?1", rusqlite::params![id], |row| row.get::<_, i64>(0))
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "Task not found"}));
    }

    if let Some(ref v) = body.project_id {
        conn.execute("UPDATE tasks SET project_id = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ref v) = body.name {
        conn.execute("UPDATE tasks SET name = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ref v) = body.description {
        conn.execute("UPDATE tasks SET description = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ref v) = body.priority {
        conn.execute("UPDATE tasks SET priority = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ref v) = body.status {
        conn.execute("UPDATE tasks SET status = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ref v) = body.due_date {
        let normalized: Option<String> = if v.is_empty() { None } else { Some(v.clone()) };
        conn.execute("UPDATE tasks SET due_date = ?1 WHERE id = ?2", rusqlite::params![normalized, id]).unwrap();
    }
    if let Some(v) = body.order_index {
        conn.execute("UPDATE tasks SET order_index = ?1 WHERE id = ?2", rusqlite::params![v, id]).unwrap();
    }
    if let Some(ids) = &body.file_ids {
        conn.execute("DELETE FROM task_files WHERE task_id = ?1", rusqlite::params![id]).unwrap();
        for fid in ids.iter().filter(|s| !s.is_empty()) {
            conn.execute(
                "INSERT OR IGNORE INTO task_files (task_id, file_id) VALUES (?1, ?2)",
                rusqlite::params![id, fid],
            ).ok();
        }
    }

    let mut task = conn
        .query_row(
            "SELECT id, project_id, name, description, priority, status, due_date, order_index, created_at FROM tasks WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok(Task {
                id: row.get(0)?, project_id: row.get(1)?, name: row.get(2)?,
                description: row.get(3)?, priority: row.get(4)?, status: row.get(5)?,
                due_date: row.get(6)?, order_index: row.get(7)?, created_at: row.get(8)?,
                files: Vec::new(),
            }),
        )
        .unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT pf.id, pf.name, pf.url, pf.file_type
             FROM task_files tf
             JOIN project_files pf ON pf.id = tf.file_id
             WHERE tf.task_id = ?1
             ORDER BY pf.created_at ASC",
        )
        .unwrap();
    task.files = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok(TaskFile {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                file_type: row.get(3)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(task)
}

async fn delete_task(data: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();
    let rows = conn.execute("DELETE FROM tasks WHERE id = ?1", rusqlite::params![id]).unwrap();
    if rows == 0 {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "Task not found"}));
    }
    HttpResponse::Ok().json(serde_json::json!({"deleted": id}))
}

// ── AI ──

#[derive(Debug, Deserialize)]
struct GenerateTasksBody {
    count: Option<usize>,
    pdf_base64: Option<String>,
    start_date: Option<String>,
    deadline: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectFromPdfBody {
    pdf_base64: String,
    count: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct BulkCreateTasksBody {
    project_id: String,
    tasks: Vec<BulkTaskItem>,
}

#[derive(Debug, Deserialize)]
struct BulkTaskItem {
    name: String,
    description: Option<String>,
    priority: Option<String>,
    due_date: Option<String>,
}

async fn generate_tasks_ai(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<GenerateTasksBody>,
) -> HttpResponse {
    let id = path.into_inner();
    let count = body.count.unwrap_or(5).clamp(1, 15);

    let (project, existing) = {
        let conn = data.db.lock().unwrap();
        let project = conn.query_row(
            "SELECT id, name, course, description, deadline, created_at FROM projects WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok(Project {
                id: row.get(0)?, name: row.get(1)?, course: row.get(2)?,
                description: row.get(3)?, deadline: row.get(4)?, created_at: row.get(5)?,
            }),
        );
        let project = match project {
            Ok(p) => p,
            Err(_) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Project not found"})),
        };

        let mut stmt = conn.prepare(
            "SELECT name, status FROM tasks WHERE project_id = ?1 ORDER BY created_at ASC",
        ).unwrap();
        let existing: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![id], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        (project, existing)
    };

    let provider = match ai::provider_from_env() {
        Ok(p) => p,
        Err(e) => return HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({"error": e.to_string()})),
    };

    let existing_refs: Vec<ExistingTask> = existing
        .iter()
        .map(|(n, s)| ExistingTask { name: n, status: s })
        .collect();

    let effective_deadline = body
        .deadline
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&project.deadline);
    let start_ref = body.start_date.as_deref().filter(|s| !s.is_empty());

    let ctx = ProjectContext {
        name: &project.name,
        course: &project.course,
        description: &project.description,
        deadline: effective_deadline,
        start_date: start_ref,
        existing_tasks: existing_refs,
    };

    match provider
        .generate_tasks(ctx, count, body.pdf_base64.as_deref())
        .await
    {
        Ok(tasks) => HttpResponse::Ok().json(serde_json::json!({
            "project_id": project.id,
            "suggestions": tasks,
        })),
        Err(e) => HttpResponse::BadGateway()
            .json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn manual_prompt_tasks(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<GenerateTasksBody>,
) -> HttpResponse {
    let id = path.into_inner();
    let count = body.count.unwrap_or(5).clamp(1, 15);

    let conn = data.db.lock().unwrap();
    let project = match conn.query_row(
        "SELECT id, name, course, description, deadline, created_at FROM projects WHERE id = ?1",
        rusqlite::params![id],
        |row| Ok(Project {
            id: row.get(0)?, name: row.get(1)?, course: row.get(2)?,
            description: row.get(3)?, deadline: row.get(4)?, created_at: row.get(5)?,
        }),
    ) {
        Ok(p) => p,
        Err(_) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Project not found"})),
    };

    let mut stmt = conn.prepare(
        "SELECT name, status FROM tasks WHERE project_id = ?1 ORDER BY created_at ASC",
    ).unwrap();
    let existing: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![id], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let existing_refs: Vec<ExistingTask> = existing
        .iter()
        .map(|(n, s)| ExistingTask { name: n, status: s })
        .collect();

    let effective_deadline = body
        .deadline
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&project.deadline);
    let start_ref = body.start_date.as_deref().filter(|s| !s.is_empty());

    let ctx = ProjectContext {
        name: &project.name,
        course: &project.course,
        description: &project.description,
        deadline: effective_deadline,
        start_date: start_ref,
        existing_tasks: existing_refs,
    };

    HttpResponse::Ok().json(serde_json::json!({
        "prompt": ai::build_manual_tasks_prompt(&ctx, count),
    }))
}

async fn manual_prompt_project(body: web::Json<GenerateTasksBody>) -> HttpResponse {
    let count = body.count.unwrap_or(6).clamp(1, 15);
    HttpResponse::Ok().json(serde_json::json!({
        "prompt": ai::build_manual_project_prompt(count),
    }))
}

async fn ai_status() -> HttpResponse {
    let configured = ai::provider_from_env().is_ok();
    HttpResponse::Ok().json(serde_json::json!({ "configured": configured }))
}

async fn project_from_pdf(body: web::Json<ProjectFromPdfBody>) -> HttpResponse {
    let provider = match ai::provider_from_env() {
        Ok(p) => p,
        Err(e) => return HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({"error": e.to_string()})),
    };

    let count = body.count.unwrap_or(6).clamp(1, 15);
    match provider.project_from_pdf(&body.pdf_base64, count).await {
        Ok(gp) => HttpResponse::Ok().json(gp),
        Err(e) => HttpResponse::BadGateway()
            .json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn bulk_create_tasks(
    data: web::Data<AppState>,
    body: web::Json<BulkCreateTasksBody>,
) -> HttpResponse {
    let conn = data.db.lock().unwrap();

    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE id = ?1",
            rusqlite::params![body.project_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Project not found"}));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let mut next_order = next_order_index(&conn, &body.project_id);
    let mut created: Vec<Task> = Vec::with_capacity(body.tasks.len());

    for item in &body.tasks {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            project_id: body.project_id.clone(),
            name: item.name.clone(),
            description: item.description.clone().unwrap_or_default(),
            priority: item.priority.clone().unwrap_or_else(|| "medium".to_string()),
            status: "todo".to_string(),
            due_date: item.due_date.clone().filter(|s| !s.is_empty()),
            order_index: next_order,
            created_at: now.clone(),
            files: Vec::new(),
        };
        conn.execute(
            "INSERT INTO tasks (id, project_id, name, description, priority, status, due_date, order_index, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                task.id, task.project_id, task.name, task.description, task.priority,
                task.status, task.due_date, task.order_index, task.created_at
            ],
        ).unwrap();
        next_order += 1;
        created.push(task);
    }

    HttpResponse::Created().json(created)
}

// ── Project Files ──

#[derive(Debug, Deserialize)]
struct UploadProjectFile {
    project_id: String,
    name: String,
    data_base64: String, // base64-encoded PDF content
}

fn files_dir() -> std::path::PathBuf {
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "projects.db".to_string());
    let parent = std::path::Path::new(&db_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let dir = parent.join("files");
    std::fs::create_dir_all(&dir).ok();
    dir
}

async fn get_project_files(data: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let project_id = path.into_inner();

    let mut stmt = conn
        .prepare("SELECT id, project_id, name, file_type, url, created_at FROM project_files WHERE project_id = ?1 ORDER BY created_at ASC")
        .unwrap();

    let files: Vec<ProjectFile> = stmt
        .query_map(rusqlite::params![project_id], |row| {
            Ok(ProjectFile {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                file_type: row.get(3)?,
                url: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(files)
}

async fn create_project_file(data: web::Data<AppState>, body: web::Json<CreateProjectFile>) -> HttpResponse {
    let conn = data.db.lock().unwrap();

    let exists: bool = conn
        .query_row("SELECT COUNT(*) FROM projects WHERE id = ?1", rusqlite::params![body.project_id], |row| row.get::<_, i64>(0))
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Project not found"}));
    }

    let file = ProjectFile {
        id: Uuid::new_v4().to_string(),
        project_id: body.project_id.clone(),
        name: body.name.clone(),
        file_type: body.file_type.clone(),
        url: body.url.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    conn.execute(
        "INSERT INTO project_files (id, project_id, name, file_type, url, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![file.id, file.project_id, file.name, file.file_type, file.url, file.created_at],
    ).unwrap();

    HttpResponse::Created().json(file)
}

async fn upload_project_file(data: web::Data<AppState>, body: web::Json<UploadProjectFile>) -> HttpResponse {
    let conn = data.db.lock().unwrap();

    let exists: bool = conn
        .query_row("SELECT COUNT(*) FROM projects WHERE id = ?1", rusqlite::params![body.project_id], |row| row.get::<_, i64>(0))
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Project not found"}));
    }

    use base64::Engine;
    let decoded = match base64::engine::general_purpose::STANDARD.decode(&body.data_base64) {
        Ok(d) => d,
        Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid base64 data"})),
    };

    let file_id = Uuid::new_v4().to_string();
    let filename = format!("{}.pdf", file_id);
    let dir = files_dir();
    let filepath = dir.join(&filename);

    if let Err(e) = std::fs::write(&filepath, &decoded) {
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to write file: {}", e)}));
    }

    let file = ProjectFile {
        id: file_id,
        project_id: body.project_id.clone(),
        name: body.name.clone(),
        file_type: "pdf".to_string(),
        url: format!("/api/files/{}", filename),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    conn.execute(
        "INSERT INTO project_files (id, project_id, name, file_type, url, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![file.id, file.project_id, file.name, file.file_type, file.url, file.created_at],
    ).unwrap();

    HttpResponse::Created().json(file)
}

async fn serve_file(path: web::Path<String>) -> HttpResponse {
    let filename = path.into_inner();
    // Prevent directory traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid filename"}));
    }
    let dir = files_dir();
    let filepath = dir.join(&filename);
    match std::fs::read(&filepath) {
        Ok(data) => HttpResponse::Ok()
            .content_type("application/pdf")
            .append_header(("Content-Disposition", format!("inline; filename=\"{}\"", filename)))
            .body(data),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({"error": "File not found"})),
    }
}

async fn delete_project_file(data: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();

    // Get the file info before deleting (to clean up from disk)
    let file_info = conn.query_row(
        "SELECT file_type, url FROM project_files WHERE id = ?1",
        rusqlite::params![id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );

    let rows = conn.execute("DELETE FROM project_files WHERE id = ?1", rusqlite::params![id]).unwrap();
    if rows == 0 {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "File not found"}));
    }

    // Clean up from disk if it's a PDF
    if let Ok((file_type, url)) = file_info {
        if file_type == "pdf" {
            if let Some(filename) = url.strip_prefix("/api/files/") {
                let filepath = files_dir().join(filename);
                std::fs::remove_file(filepath).ok();
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({"deleted": id}))
}

// ── Courses ──

async fn get_courses(data: web::Data<AppState>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT DISTINCT course FROM projects ORDER BY course ASC")
        .unwrap();

    let courses: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(courses)
}

// ── Main ──

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _ = dotenvy::dotenv();
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "projects.db".to_string());
    let conn = Connection::open(&db_path).expect("Failed to open database");
    init_db(&conn);

    let data = web::Data::new(AppState {
        db: Mutex::new(conn),
    });

    println!("Server running at http://localhost:8080");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(data.clone())
            .app_data(web::JsonConfig::default().limit(20 * 1024 * 1024))
            .route("/api/projects", web::get().to(get_projects))
            .route("/api/projects", web::post().to(create_project))
            .route("/api/projects/{id}", web::put().to(update_project))
            .route("/api/projects/{id}", web::delete().to(delete_project))
            .route("/api/tasks", web::get().to(get_tasks))
            .route("/api/tasks", web::post().to(create_task))
            .route("/api/tasks/{id}", web::put().to(update_task))
            .route("/api/tasks/{id}", web::delete().to(delete_task))
            .route("/api/tasks/bulk", web::post().to(bulk_create_tasks))
            .route("/api/projects/{id}/ai/generate-tasks", web::post().to(generate_tasks_ai))
            .route("/api/ai/project-from-pdf", web::post().to(project_from_pdf))
            .route("/api/projects/{id}/ai/manual-prompt", web::post().to(manual_prompt_tasks))
            .route("/api/ai/manual-prompt/project", web::post().to(manual_prompt_project))
            .route("/api/ai/status", web::get().to(ai_status))
            .route("/api/courses", web::get().to(get_courses))
            .route("/api/projects/{id}/files", web::get().to(get_project_files))
            .route("/api/project-files", web::post().to(create_project_file))
            .route("/api/project-files/upload", web::post().to(upload_project_file))
            .route("/api/project-files/{id}", web::delete().to(delete_project_file))
            .route("/api/files/{filename}", web::get().to(serve_file))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
