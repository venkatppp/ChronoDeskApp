//! Token Counter - Estimate token usage for conversation management

use super::models::LLMMessage;

/// Token counter for managing context windows
pub struct TokenCounter;

impl TokenCounter {
    /// Estimates token count for a message (simple approximation)
    /// Real implementation would use tiktoken or similar
    pub fn estimate_tokens(text: &str) -> usize {
        // Rough approximation: 1 token ≈ 4 characters
        // This is conservative (overestimates slightly)
        (text.len() as f32 / 3.5) as usize
    }

    /// Estimates token count for a list of messages
    pub fn estimate_messages_tokens(messages: &[LLMMessage]) -> usize {
        let mut total = 0;
        for msg in messages {
            // Count role tokens
            total += Self::estimate_tokens(&msg.role);
            // Count content tokens
            total += Self::estimate_tokens(&msg.content);
            // Add overhead for message formatting (approximately 4 tokens per message)
            total += 4;
        }
        total
    }

    /// Truncates messages to fit within context window
    pub fn truncate_to_context(
        messages: &[LLMMessage],
        context_window: usize,
        reserved_tokens: usize,
    ) -> Vec<LLMMessage> {
        let max_tokens = context_window.saturating_sub(reserved_tokens);

        if messages.is_empty() {
            return Vec::new();
        }

        // Always keep the system message (if present) and the last user message
        let mut result = Vec::new();
        let mut current_tokens = 0;

        // Keep system message if present
        if messages.first().map(|m| m.role.as_str()) == Some("system") {
            let system_msg = &messages[0];
            let tokens = Self::estimate_tokens(&system_msg.content) + 4;
            result.push(system_msg.clone());
            current_tokens += tokens;
        }

        // Keep the most recent messages that fit
        for msg in messages.iter().rev() {
            if msg.role == "system" {
                continue; // Already added
            }

            let msg_tokens =
                Self::estimate_tokens(&msg.content) + Self::estimate_tokens(&msg.role) + 4;

            if current_tokens + msg_tokens <= max_tokens {
                result.insert(if result.is_empty() { 0 } else { 1 }, msg.clone());
                current_tokens += msg_tokens;
            } else {
                break;
            }
        }

        result
    }

    /// Checks if messages fit within context window
    pub fn fits_in_context(messages: &[LLMMessage], context_window: usize) -> bool {
        Self::estimate_messages_tokens(messages) <= context_window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert!(TokenCounter::estimate_tokens("Hello, world!") > 0);
        assert!(TokenCounter::estimate_tokens("") == 0);
    }

    #[test]
    fn test_estimate_messages_tokens() {
        let messages = vec![
            LLMMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            LLMMessage {
                role: "assistant".to_string(),
                content: "Hi there!".to_string(),
            },
        ];

        let tokens = TokenCounter::estimate_messages_tokens(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn test_truncate_to_context() {
        let messages = vec![
            LLMMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
            },
            LLMMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            LLMMessage {
                role: "assistant".to_string(),
                content: "Hi!".to_string(),
            },
            LLMMessage {
                role: "user".to_string(),
                content: "How are you?".to_string(),
            },
        ];

        let truncated = TokenCounter::truncate_to_context(&messages, 100, 50);
        assert!(!truncated.is_empty());
        // System message should be preserved
        assert_eq!(truncated[0].role, "system");
    }
}
