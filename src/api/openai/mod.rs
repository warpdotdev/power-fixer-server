//! OpenAI integration for summarizing GitHub issues.

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use std::env;

const MODEL: &str = "gpt-4o-mini";
const MAX_TOKENS: u32 = 100;

#[derive(Debug)]
pub struct OpenAIClient {
    client: Client<OpenAIConfig>,
}

impl OpenAIClient {
    pub fn new() -> Result<Self, String> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY environment variable not set".to_string())?;

        if api_key.is_empty() {
            return Err("OPENAI_API_KEY is empty".to_string());
        }

        let mut config = OpenAIConfig::new().with_api_key(api_key);
        if let Ok(base_url) = env::var("OPENAI_API_BASE_URL") {
            if !base_url.trim().is_empty() {
                config = config.with_api_base(base_url);
            }
        }
        let client = Client::with_config(config);

        Ok(Self { client })
    }

    pub async fn summarize_issue(&self, issue_body: &str) -> Result<String, String> {
        if issue_body.trim().is_empty() {
            return Ok(String::new());
        }

        let system_msg: ChatCompletionRequestMessage =
            ChatCompletionRequestSystemMessageArgs::default()
                .content(
                    "You are a helpful assistant that summarizes GitHub issues. \
                 Provide a brief 1-2 sentence summary of the issue that captures \
                 the main problem or request. Be concise and focus on what the user \
                 is experiencing or asking for. Do not include markdown formatting.",
                )
                .build()
                .map_err(|e| format!("Failed to build system message: {}", e))?
                .into();

        let user_msg: ChatCompletionRequestMessage =
            ChatCompletionRequestUserMessageArgs::default()
                .content(issue_body)
                .build()
                .map_err(|e| format!("Failed to build user message: {}", e))?
                .into();

        let request = CreateChatCompletionRequestArgs::default()
            .model(MODEL)
            .max_tokens(MAX_TOKENS)
            .messages(vec![system_msg, user_msg])
            .build()
            .map_err(|e| format!("Failed to build chat completion request: {}", e))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| format!("OpenAI API error: {}", e))?;

        let summary = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(summary.trim().to_string())
    }
}

pub async fn summarize_or_truncate(
    openai_client: Option<&OpenAIClient>,
    text: &str,
    max_len: usize,
) -> String {
    if text.trim().is_empty() {
        return String::new();
    }

    if let Some(client) = openai_client {
        match client.summarize_issue(text).await {
            Ok(summary) if !summary.is_empty() => return summary,
            Ok(_) => log::debug!("OpenAI returned empty summary, falling back to truncation"),
            Err(e) => log::warn!(
                "OpenAI summarization failed: {}, falling back to truncation",
                e
            ),
        }
    }

    crate::api::types::truncate_text(text, max_len)
}
