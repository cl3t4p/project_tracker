use async_trait::async_trait;
use serde_json::{json, Value};

use super::{
    build_pdf_project_prompt, build_prompt, AiError, GeneratedProject, LlmProvider,
    ProjectContext, SuggestedTask,
};

const API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    fn task_schema() -> Value {
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
            "required": ["name", "description", "priority", "due_date"],
            "additionalProperties": false
        })
    }

    fn user_message(text: &str, pdf_base64: Option<&str>) -> Value {
        match pdf_base64 {
            Some(b64) => json!({
                "role": "user",
                "content": [
                    {
                        "type": "file",
                        "file": {
                            "filename": "brief.pdf",
                            "file_data": format!("data:application/pdf;base64,{}", b64)
                        }
                    },
                    {"type": "text", "text": text}
                ]
            }),
            None => json!({"role": "user", "content": text}),
        }
    }

    async fn call_tool(
        &self,
        messages: Value,
        tool_name: &str,
        parameters: Value,
    ) -> Result<Value, AiError> {
        let body = json!({
            "model": self.model,
            "messages": messages,
            "tools": [{
                "type": "function",
                "function": {
                    "name": tool_name,
                    "parameters": parameters,
                }
            }],
            "tool_choice": {"type": "function", "function": {"name": tool_name}}
        });

        let res = self
            .client
            .post(API_URL)
            .bearer_auth(&self.api_key)
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

        let payload: Value = res.json().await?;
        let args_str = payload
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("tool_calls"))
            .and_then(|t| t.get(0))
            .and_then(|t| t.get("function"))
            .and_then(|f| f.get("arguments"))
            .and_then(|a| a.as_str())
            .ok_or_else(|| AiError::Malformed("missing tool_call arguments".into()))?;

        Ok(serde_json::from_str(args_str)?)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn generate_tasks(
        &self,
        ctx: ProjectContext<'_>,
        count: usize,
        pdf_base64: Option<&str>,
    ) -> Result<Vec<SuggestedTask>, AiError> {
        let prompt = build_prompt(&ctx, count, pdf_base64.is_some());
        let messages = json!([Self::user_message(&prompt, pdf_base64)]);

        let parameters = json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": Self::task_schema()
                }
            },
            "required": ["tasks"],
            "additionalProperties": false
        });

        let args = self.call_tool(messages, "create_tasks", parameters).await?;
        let tasks = args
            .get("tasks")
            .ok_or_else(|| AiError::Malformed("arguments missing 'tasks'".into()))?;
        Ok(serde_json::from_value(tasks.clone())?)
    }

    async fn project_from_pdf(
        &self,
        pdf_base64: &str,
        task_count: usize,
    ) -> Result<GeneratedProject, AiError> {
        let prompt = build_pdf_project_prompt(task_count);
        let messages = json!([Self::user_message(&prompt, Some(pdf_base64))]);

        let parameters = json!({
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
                    "items": Self::task_schema()
                }
            },
            "required": ["name", "course", "description", "tasks"],
            "additionalProperties": false
        });

        let args = self.call_tool(messages, "create_project", parameters).await?;
        Ok(serde_json::from_value(args)?)
    }
}
