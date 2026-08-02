//! LLM IPC Commands - LLM provider configuration

use std::sync::Arc;
use tauri::State;

use crate::llm::{LLMService, LLMSettings};

/// Gets current LLM settings
#[tauri::command]
pub async fn llm_get_settings(service: State<'_, Arc<LLMService>>) -> Result<LLMSettings, String> {
    service.get_settings().await.map_err(|e| e.to_string())
}

/// Updates LLM settings
#[tauri::command]
pub async fn llm_update_settings(
    service: State<'_, Arc<LLMService>>,
    settings: LLMSettings,
) -> Result<(), String> {
    service
        .update_settings(&settings)
        .await
        .map_err(|e| e.to_string())
}

/// Tests LLM connection
#[tauri::command]
pub async fn llm_test_connection(service: State<'_, Arc<LLMService>>) -> Result<(), String> {
    service.test_connection().await
}

/// Checks if LLM is configured
#[tauri::command]
pub async fn llm_is_configured(service: State<'_, Arc<LLMService>>) -> Result<bool, String> {
    Ok(service.is_configured())
}
