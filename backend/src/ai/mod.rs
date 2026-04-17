use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod claude;
pub mod openai;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SuggestedTask {
    pub name: String,
    pub description: String,
    pub priority: String,
    #[serde(default)]
    pub due_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneratedProject {
    pub name: String,
    pub course: String,
    pub description: String,
    pub deadline: Option<String>,
    pub tasks: Vec<SuggestedTask>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectContext<'a> {
    pub name: &'a str,
    pub course: &'a str,
    pub description: &'a str,
    pub deadline: &'a str,
    pub start_date: Option<&'a str>,
    pub existing_tasks: Vec<ExistingTask<'a>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExistingTask<'a> {
    pub name: &'a str,
    pub status: &'a str,
}

#[derive(Debug, Error)]
pub enum AiError {
    #[error("provider not configured: {0}")]
    NotConfigured(String),
    #[error("unsupported provider: {0}")]
    Unsupported(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("malformed response: {0}")]
    Malformed(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate_tasks(
        &self,
        ctx: ProjectContext<'_>,
        count: usize,
        pdf_base64: Option<&str>,
    ) -> Result<Vec<SuggestedTask>, AiError>;

    async fn project_from_pdf(
        &self,
        pdf_base64: &str,
        task_count: usize,
    ) -> Result<GeneratedProject, AiError>;
}

pub fn provider_from_env() -> Result<Box<dyn LlmProvider>, AiError> {
    let name = std::env::var("AI_PROVIDER").unwrap_or_else(|_| "claude".to_string());
    match name.to_lowercase().as_str() {
        "claude" | "anthropic" => {
            let key = std::env::var("AI_API_KEY")
                .map_err(|_| AiError::NotConfigured("AI_API_KEY".into()))?;
            let model = std::env::var("AI_MODEL")
                .unwrap_or_else(|_| "claude-haiku-4-5-20251001".to_string());
            Ok(Box::new(claude::ClaudeProvider::new(key, model)))
        }
        "openai" | "chatgpt" => {
            let key = std::env::var("AI_API_KEY")
                .map_err(|_| AiError::NotConfigured("AI_API_KEY".into()))?;
            let model = std::env::var("AI_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".to_string());
            Ok(Box::new(openai::OpenAiProvider::new(key, model)))
        }
        other => Err(AiError::Unsupported(other.to_string())),
    }
}

pub fn build_prompt(ctx: &ProjectContext<'_>, count: usize, has_pdf: bool) -> String {
    let existing = if ctx.existing_tasks.is_empty() {
        "(none yet)".to_string()
    } else {
        ctx.existing_tasks
            .iter()
            .map(|t| format!("- [{}] {}", t.status, t.name))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let pdf_note = if has_pdf {
        "\nA project brief PDF is attached. Use its contents to inform the tasks.\n"
    } else {
        ""
    };

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let start = ctx.start_date.unwrap_or(today.as_str());

    format!(
        "You are helping a university student plan a project.\n\n\
        Today's date: {today}\n\
        Project: {name}\n\
        Course: {course}\n\
        Start date (when the student will begin): {start}\n\
        Final deadline: {deadline}\n\
        Description: {description}\n\n\
        Existing tasks:\n{existing}\n{pdf_note}\n\
        Generate up to {count} concrete, actionable tasks that move this project forward. \
        Do NOT duplicate existing tasks. \
        Return the tasks in the order they should be done (earliest first). \
        For each task, set 'due_date' to the day the student should finish it (YYYY-MM-DD). \
        Space due dates sensibly between the start date and the final deadline so the student has a realistic timeline. \
        No task's due date may be after the final deadline. \
        Each task should have a short name (max ~60 chars), a one-or-two-sentence description, \
        and a priority of 'low', 'medium', or 'high'.",
        today = today,
        name = ctx.name,
        course = ctx.course,
        start = start,
        deadline = ctx.deadline,
        description = if ctx.description.is_empty() { "(none)" } else { ctx.description },
        existing = existing,
        pdf_note = pdf_note,
        count = count,
    )
}

pub fn build_manual_tasks_prompt(ctx: &ProjectContext<'_>, count: usize) -> String {
    format!(
        "{}\n\n\
        Respond with ONLY raw JSON (no markdown, no prose) matching exactly this shape:\n\
        {{\"tasks\": [{{\"name\": \"...\", \"description\": \"...\", \
        \"priority\": \"low|medium|high\", \"due_date\": \"YYYY-MM-DD\"}}]}}\n\
        The array order is the recommended execution order (first task = do first).",
        build_prompt(ctx, count, false)
    )
}

pub fn build_manual_project_prompt(count: usize) -> String {
    format!(
        "{}\n\n\
        If a project-brief PDF is attached to this chat, use its contents. \
        Respond with ONLY raw JSON (no markdown, no prose) matching exactly this shape:\n\
        {{\"name\": \"...\", \"course\": \"...\", \"description\": \"...\", \
        \"deadline\": \"YYYY-MM-DD or null\", \
        \"tasks\": [{{\"name\": \"...\", \"description\": \"...\", \
        \"priority\": \"low|medium|high\", \"due_date\": \"YYYY-MM-DD or null\"}}]}}",
        build_pdf_project_prompt(count)
    )
}

pub fn build_pdf_project_prompt(count: usize) -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    format!(
        "You are reading a university project brief (attached as PDF). \
        Extract the project metadata and propose an initial task list.\n\n\
        Today's date: {today}\n\
        Return:\n\
        - name: short project title (max ~60 chars)\n\
        - course: course code or name if identifiable, else 'Unknown'\n\
        - description: 1-3 sentence summary of the project\n\
        - deadline: final submission date in YYYY-MM-DD if present in the brief, otherwise null\n\
        - tasks: up to {count} concrete, actionable tasks in execution order (first = do first), \
          each with a short name, a one-or-two-sentence description, \
          a priority of 'low', 'medium', or 'high', \
          and a 'due_date' (YYYY-MM-DD) that gives the student a realistic target finish date \
          between today and the project deadline. No due_date may be after the project deadline.",
        today = today,
        count = count,
    )
}
