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
    file_type: String, // "link", "pdf", "file"
    url: String,
    created_at: String,
    #[serde(default = "default_category")]
    category: String, // "course", "assignment", "lab", "exam", "other"
    #[serde(default)]
    subsection: Option<String>,
    #[serde(default)]
    order_index: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Subsection {
    category: String,
    name: String,
    created_at: String,
    #[serde(default)]
    order_index: i64,
}

#[derive(Debug, Deserialize)]
struct ReorderSubsectionsBody {
    category: String,
    names: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ReorderFilesBody {
    ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreateSubsectionBody {
    category: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct SubsectionQuery {
    category: String,
    name: String,
}

fn default_category() -> String {
    "other".to_string()
}

#[derive(Debug, Serialize, Clone)]
struct ProjectFileWithContext {
    #[serde(flatten)]
    file: ProjectFile,
    project_name: String,
    course: String,
}

#[derive(Debug, Serialize, Clone)]
struct CourseInfo {
    name: String,
    exam_deadline: Option<String>,
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
    category: Option<String>,
    subsection: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateProjectFileBody {
    name: Option<String>,
    category: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    subsection: Option<Option<String>>,
}

// Distinguish between missing field and explicit null so we can clear the subsection.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
struct UpdateCourseBody {
    exam_deadline: Option<String>,
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
        CREATE TABLE IF NOT EXISTS course_meta (
            name TEXT PRIMARY KEY,
            exam_deadline TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS subsections (
            category TEXT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (category, name)
        );
        PRAGMA foreign_keys = ON;",
    )
    .expect("Failed to initialize database");

    ensure_column(conn, "tasks", "due_date", "TEXT");
    ensure_column(conn, "tasks", "order_index", "INTEGER NOT NULL DEFAULT 0");
    ensure_column(conn, "tasks", "file_id", "TEXT");
    ensure_column(conn, "project_files", "category", "TEXT NOT NULL DEFAULT 'other'");
    ensure_column(conn, "project_files", "subsection", "TEXT");
    ensure_column(conn, "project_files", "order_index", "INTEGER NOT NULL DEFAULT 0");
    ensure_column(conn, "subsections", "order_index", "INTEGER NOT NULL DEFAULT 0");

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
    data_base64: String, // base64-encoded file content
    filename: Option<String>, // original filename (used to preserve extension)
    category: Option<String>,
    subsection: Option<String>,
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
        .prepare("SELECT id, project_id, name, file_type, url, created_at, COALESCE(category, 'other'), subsection, COALESCE(order_index, 0) FROM project_files WHERE project_id = ?1 ORDER BY order_index ASC, created_at ASC")
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
                category: row.get(6)?,
                subsection: row.get(7).ok(),
                order_index: row.get(8)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(files)
}

async fn get_all_files(data: web::Data<AppState>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT pf.id, pf.project_id, pf.name, pf.file_type, pf.url, pf.created_at, \
                    COALESCE(pf.category, 'other'), pf.subsection, COALESCE(pf.order_index, 0), p.name, p.course \
             FROM project_files pf JOIN projects p ON p.id = pf.project_id \
             ORDER BY pf.order_index ASC, pf.created_at ASC",
        )
        .unwrap();

    let files: Vec<ProjectFileWithContext> = stmt
        .query_map([], |row| {
            Ok(ProjectFileWithContext {
                file: ProjectFile {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    file_type: row.get(3)?,
                    url: row.get(4)?,
                    created_at: row.get(5)?,
                    category: row.get(6)?,
                    subsection: row.get(7).ok(),
                    order_index: row.get(8)?,
                },
                project_name: row.get(9)?,
                course: row.get(10)?,
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

    let order_index = next_file_order(&conn);
    let file = ProjectFile {
        id: Uuid::new_v4().to_string(),
        project_id: body.project_id.clone(),
        name: body.name.clone(),
        file_type: body.file_type.clone(),
        url: body.url.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        category: body.category.clone().unwrap_or_else(default_category),
        subsection: body.subsection.clone().filter(|s| !s.is_empty()),
        order_index,
    };

    conn.execute(
        "INSERT INTO project_files (id, project_id, name, file_type, url, created_at, category, subsection, order_index) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![file.id, file.project_id, file.name, file.file_type, file.url, file.created_at, file.category, file.subsection, file.order_index],
    ).unwrap();

    HttpResponse::Created().json(file)
}

fn next_file_order(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT COALESCE(MAX(order_index), -1) + 1 FROM project_files",
        [],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
}

async fn reorder_project_files(
    data: web::Data<AppState>,
    body: web::Json<ReorderFilesBody>,
) -> HttpResponse {
    let mut conn = data.db.lock().unwrap();
    let tx = conn.transaction().unwrap();
    for (i, id) in body.ids.iter().enumerate() {
        tx.execute(
            "UPDATE project_files SET order_index = ?1 WHERE id = ?2",
            rusqlite::params![i as i64, id],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    HttpResponse::Ok().json(serde_json::json!({"ok": true, "count": body.ids.len()}))
}

async fn update_project_file(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateProjectFileBody>,
) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();

    if let Some(ref name) = body.name {
        conn.execute(
            "UPDATE project_files SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, id],
        )
        .unwrap();
    }
    if let Some(ref category) = body.category {
        conn.execute(
            "UPDATE project_files SET category = ?1 WHERE id = ?2",
            rusqlite::params![category, id],
        )
        .unwrap();
    }
    if let Some(ref sub) = body.subsection {
        let value = sub.as_ref().map(|s| s.as_str()).filter(|s| !s.is_empty());
        conn.execute(
            "UPDATE project_files SET subsection = ?1 WHERE id = ?2",
            rusqlite::params![value, id],
        )
        .unwrap();
    }

    let result = conn.query_row(
        "SELECT id, project_id, name, file_type, url, created_at, COALESCE(category, 'other'), subsection, COALESCE(order_index, 0) FROM project_files WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(ProjectFile {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                file_type: row.get(3)?,
                url: row.get(4)?,
                created_at: row.get(5)?,
                category: row.get(6)?,
                subsection: row.get(7).ok(),
                order_index: row.get(8)?,
            })
        },
    );
    match result {
        Ok(file) => HttpResponse::Ok().json(file),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({"error": "File not found"})),
    }
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
    let ext = body
        .filename
        .as_deref()
        .and_then(|f| std::path::Path::new(f).extension().and_then(|e| e.to_str()))
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "pdf".to_string());
    let stored_name = format!("{}.{}", file_id, ext);
    let file_type = if ext == "pdf" { "pdf" } else { "file" };
    let dir = files_dir();
    let filepath = dir.join(&stored_name);

    if let Err(e) = std::fs::write(&filepath, &decoded) {
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to write file: {}", e)}));
    }

    let order_index = next_file_order(&conn);
    let file = ProjectFile {
        id: file_id,
        project_id: body.project_id.clone(),
        name: body.name.clone(),
        file_type: file_type.to_string(),
        url: format!("/api/files/{}", stored_name),
        created_at: chrono::Utc::now().to_rfc3339(),
        category: body.category.clone().unwrap_or_else(default_category),
        subsection: body.subsection.clone().filter(|s| !s.is_empty()),
        order_index,
    };

    conn.execute(
        "INSERT INTO project_files (id, project_id, name, file_type, url, created_at, category, subsection, order_index) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![file.id, file.project_id, file.name, file.file_type, file.url, file.created_at, file.category, file.subsection, file.order_index],
    ).unwrap();

    HttpResponse::Created().json(file)
}

fn content_type_for(ext: &str) -> &'static str {
    match ext {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "txt" | "md" | "log" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html; charset=utf-8",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" | "tgz" => "application/gzip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
}

async fn serve_file(path: web::Path<String>) -> HttpResponse {
    let filename = path.into_inner();
    // Prevent directory traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid filename"}));
    }
    let dir = files_dir();
    let filepath = dir.join(&filename);
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match std::fs::read(&filepath) {
        Ok(data) => HttpResponse::Ok()
            .content_type(content_type_for(&ext))
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

    // Clean up from disk for any stored file (not links)
    if let Ok((file_type, url)) = file_info {
        if file_type != "link" {
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

async fn get_courses_detailed(data: web::Data<AppState>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT p.course, cm.exam_deadline \
             FROM projects p LEFT JOIN course_meta cm ON cm.name = p.course \
             ORDER BY p.course ASC",
        )
        .unwrap();

    let courses: Vec<CourseInfo> = stmt
        .query_map([], |row| {
            Ok(CourseInfo {
                name: row.get(0)?,
                exam_deadline: row.get(1).ok(),
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(courses)
}

// ── Subsections ──

async fn get_subsections(data: web::Data<AppState>) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT category, name, created_at, COALESCE(order_index, 0) FROM subsections ORDER BY category ASC, order_index ASC, name ASC")
        .unwrap();

    let rows: Vec<Subsection> = stmt
        .query_map([], |row| {
            Ok(Subsection {
                category: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                order_index: row.get(3)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(rows)
}

async fn create_subsection(
    data: web::Data<AppState>,
    body: web::Json<CreateSubsectionBody>,
) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let name = body.name.trim();
    let category = body.category.trim();
    if name.is_empty() || category.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "category and name required"}));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let order_index: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(order_index), -1) + 1 FROM subsections WHERE category = ?1",
            rusqlite::params![category],
            |row| row.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT OR IGNORE INTO subsections (category, name, created_at, order_index) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![category, name, now, order_index],
    )
    .unwrap();
    HttpResponse::Created().json(Subsection {
        category: category.to_string(),
        name: name.to_string(),
        created_at: now,
        order_index,
    })
}

async fn reorder_subsections(
    data: web::Data<AppState>,
    body: web::Json<ReorderSubsectionsBody>,
) -> HttpResponse {
    let mut conn = data.db.lock().unwrap();
    let tx = conn.transaction().unwrap();
    for (i, name) in body.names.iter().enumerate() {
        tx.execute(
            "UPDATE subsections SET order_index = ?1 WHERE category = ?2 AND name = ?3",
            rusqlite::params![i as i64, body.category, name],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    HttpResponse::Ok().json(serde_json::json!({"ok": true, "count": body.names.len()}))
}

async fn delete_subsection(
    data: web::Data<AppState>,
    query: web::Query<SubsectionQuery>,
) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    conn.execute(
        "DELETE FROM subsections WHERE category = ?1 AND name = ?2",
        rusqlite::params![query.category, query.name],
    )
    .unwrap();
    // Unassign any files that referenced this subsection
    conn.execute(
        "UPDATE project_files SET subsection = NULL WHERE category = ?1 AND subsection = ?2",
        rusqlite::params![query.category, query.name],
    )
    .unwrap();
    HttpResponse::Ok().json(serde_json::json!({"deleted": query.name}))
}

async fn upsert_course(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateCourseBody>,
) -> HttpResponse {
    let conn = data.db.lock().unwrap();
    let name = path.into_inner();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO course_meta (name, exam_deadline, created_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(name) DO UPDATE SET exam_deadline = excluded.exam_deadline",
        rusqlite::params![name, body.exam_deadline, now],
    )
    .unwrap();

    HttpResponse::Ok().json(CourseInfo {
        name,
        exam_deadline: body.exam_deadline.clone(),
    })
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
            // ── Project tracker API ──
            .route("/api/project_tracker/projects", web::get().to(get_projects))
            .route("/api/project_tracker/projects", web::post().to(create_project))
            .route("/api/project_tracker/projects/{id}", web::put().to(update_project))
            .route("/api/project_tracker/projects/{id}", web::delete().to(delete_project))
            .route("/api/project_tracker/tasks", web::get().to(get_tasks))
            .route("/api/project_tracker/tasks", web::post().to(create_task))
            .route("/api/project_tracker/tasks/{id}", web::put().to(update_task))
            .route("/api/project_tracker/tasks/{id}", web::delete().to(delete_task))
            .route("/api/project_tracker/tasks/bulk", web::post().to(bulk_create_tasks))
            .route("/api/project_tracker/projects/{id}/ai/generate-tasks", web::post().to(generate_tasks_ai))
            .route("/api/project_tracker/ai/project-from-pdf", web::post().to(project_from_pdf))
            .route("/api/project_tracker/projects/{id}/ai/manual-prompt", web::post().to(manual_prompt_tasks))
            .route("/api/project_tracker/ai/manual-prompt/project", web::post().to(manual_prompt_project))
            .route("/api/project_tracker/ai/status", web::get().to(ai_status))
            .route("/api/project_tracker/courses", web::get().to(get_courses))
            .route("/api/project_tracker/projects/{id}/files", web::get().to(get_project_files))
            .route("/api/project_tracker/project-files", web::post().to(create_project_file))
            .route("/api/project_tracker/project-files/upload", web::post().to(upload_project_file))
            .route("/api/project_tracker/project-files/order", web::post().to(reorder_project_files))
            .route("/api/project_tracker/project-files/{id}", web::put().to(update_project_file))
            .route("/api/project_tracker/project-files/{id}", web::delete().to(delete_project_file))
            // ── Course library API ──
            .route("/api/course/list", web::get().to(get_courses_detailed))
            .route("/api/course/files", web::get().to(get_all_files))
            .route("/api/course/subsections", web::get().to(get_subsections))
            .route("/api/course/subsections", web::post().to(create_subsection))
            .route("/api/course/subsections", web::delete().to(delete_subsection))
            .route("/api/course/subsections/order", web::post().to(reorder_subsections))
            .route("/api/course/{name}", web::put().to(upsert_course))
            // ── Shared file serving ──
            .route("/api/files/{filename}", web::get().to(serve_file))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
