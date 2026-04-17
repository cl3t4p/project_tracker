use async_trait::async_trait;
use serde_json::{json, Value};

use super::{
    build_pdf_project_prompt, build_prompt, AiError, GeneratedProject, LlmProvider,
    ProjectContext, SuggestedTask,
};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    fn task_tool() -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Short task title"},
                "description": {"type": "string", "description": "Details of what to do"},
                "priority": {"type": "string", "enum": ["low", "medium", "high"]},
                "due_date": {
                    "type": ["string", "null"],
                    "description": "Target finish date (YYYY-MM-DD) on or before the project deadline"
                }
            },
            "required": ["name", "description", "priority"]
        })
    }

    fn user_message(text: &str, pdf_base64: Option<&str>) -> Value {
        match pdf_base64 {
            Some(b64) => json!({
                "role": "user",
                "content": [
                    {
                        "type": "document",
                        "source": {
                            "type": "base64",
                            "media_type": "application/pdf",
                            "data": b64
                        }
                    },
                    {"type": "text", "text": text}
                ]
            }),
            None => json!({"role": "user", "content": text}),
        }
    }

    async fn call(&self, body: Value) -> Result<Value, AiError> {
        let res = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AiError::Api {
                status: status.as_u16(),
                body,
            });
        }
        Ok(res.json().await?)
    }

    fn extract_tool_input(payload: &Value, tool_name: &str) -> Result<Value, AiError> {
        let content = payload
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or_else(|| AiError::Malformed("missing content array".into()))?;

        content
            .iter()
            .find(|b| {
                b.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                    && b.get("name").and_then(|n| n.as_str()) == Some(tool_name)
            })
            .and_then(|b| b.get("input").cloned())
            .ok_or_else(|| AiError::Malformed(format!("no tool_use block named {tool_name}")))
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn generate_tasks(
        &self,
        ctx: ProjectContext<'_>,
        count: usize,
        pdf_base64: Option<&str>,
    ) -> Result<Vec<SuggestedTask>, AiError> {
        let prompt = build_prompt(&ctx, count, pdf_base64.is_some());

        let tool = json!({
            "name": "create_tasks",
            "description": "Record a list of concrete tasks for the project",
            "input_schema": {
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "items": Self::task_tool()
                    }
                },
                "required": ["tasks"]
            }
        });

        let body = json!({
            "model": self.model,
            "max_tokens": 2048,
            "tools": [tool],
            "tool_choice": {"type": "tool", "name": "create_tasks"},
            "messages": [Self::user_message(&prompt, pdf_base64)],
        });

        let payload = self.call(body).await?;
        let tool_input = Self::extract_tool_input(&payload, "create_tasks")?;
        let tasks = tool_input
            .get("tasks")
            .ok_or_else(|| AiError::Malformed("tool_use missing 'tasks'".into()))?;
        Ok(serde_json::from_value(tasks.clone())?)
    }

    async fn project_from_pdf(
        &self,
        pdf_base64: &str,
        task_count: usize,
    ) -> Result<GeneratedProject, AiError> {
        let prompt = build_pdf_project_prompt(task_count);

        let tool = json!({
            "name": "create_project",
            "description": "Record project metadata and initial task list from the brief",
            "input_schema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "course": {"type": "string"},
                    "description": {"type": "string"},
                    "deadline": {
                        "type": ["string", "null"],
                        "description": "YYYY-MM-DD or null if unknown"
                    },
                    "tasks": {
                        "type": "array",
                        "items": Self::task_tool()
                    }
                },
                "required": ["name", "course", "description", "tasks"]
            }
        });

        let body = json!({
            "model": self.model,
            "max_tokens": 3072,
            "tools": [tool],
            "tool_choice": {"type": "tool", "name": "create_project"},
            "messages": [Self::user_message(&prompt, Some(pdf_base64))],
        });

        let payload = self.call(body).await?;
        let tool_input = Self::extract_tool_input(&payload, "create_project")?;
        Ok(serde_json::from_value(tool_input)?)
    }
}
