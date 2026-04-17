use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

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
}

#[derive(Debug, Deserialize)]
struct UpdateTaskBody {
    project_id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    priority: Option<String>,
    status: Option<String>,
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
            created_at TEXT NOT NULL
        );
        PRAGMA foreign_keys = ON;",
    )
    .expect("Failed to initialize database");
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
        .prepare("SELECT id, project_id, name, description, priority, status, created_at FROM tasks ORDER BY created_at ASC")
        .unwrap();

    let tasks: Vec<Task> = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                priority: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

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

    let task = Task {
        id: Uuid::new_v4().to_string(),
        project_id: body.project_id.clone(),
        name: body.name.clone(),
        description: body.description.clone().unwrap_or_default(),
        priority: body.priority.clone().unwrap_or_else(|| "medium".to_string()),
        status: body.status.clone().unwrap_or_else(|| "todo".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    conn.execute(
        "INSERT INTO tasks (id, project_id, name, description, priority, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![task.id, task.project_id, task.name, task.description, task.priority, task.status, task.created_at],
    ).unwrap();

    HttpResponse::Created().json(task)
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

    let task = conn
        .query_row(
            "SELECT id, project_id, name, description, priority, status, created_at FROM tasks WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok(Task { id: row.get(0)?, project_id: row.get(1)?, name: row.get(2)?, description: row.get(3)?, priority: row.get(4)?, status: row.get(5)?, created_at: row.get(6)? }),
        )
        .unwrap();

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
            .route("/api/projects", web::get().to(get_projects))
            .route("/api/projects", web::post().to(create_project))
            .route("/api/projects/{id}", web::put().to(update_project))
            .route("/api/projects/{id}", web::delete().to(delete_project))
            .route("/api/tasks", web::get().to(get_tasks))
            .route("/api/tasks", web::post().to(create_task))
            .route("/api/tasks/{id}", web::put().to(update_task))
            .route("/api/tasks/{id}", web::delete().to(delete_task))
            .route("/api/courses", web::get().to(get_courses))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
