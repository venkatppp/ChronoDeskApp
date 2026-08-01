//! Tokenizer for BERT-based models.

use std::path::Path;
use tokenizers::Tokenizer;

use crate::errors::DatabaseError;

/// BERT tokenizer for text preprocessing.
pub struct BertTokenizer {
    tokenizer: Tokenizer,
    max_length: usize,
}

impl BertTokenizer {
    /// Loads a tokenizer from a file.
    pub fn from_file(path: &Path, max_length: usize) -> Result<Self, DatabaseError> {
        let tokenizer = Tokenizer::from_file(path)
            .map_err(|e| DatabaseError::IoError(format!("Failed to load tokenizer: {}", e)))?;

        Ok(Self {
            tokenizer,
            max_length,
        })
    }

    /// Tokenizes a single text input.
    pub fn tokenize(&self, text: &str) -> Result<TokenizedInput, DatabaseError> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| DatabaseError::IoError(format!("Failed to tokenize text: {}", e)))?;

        let mut input_ids = encoding.get_ids().to_vec();
        let mut attention_mask = encoding.get_attention_mask().to_vec();

        // Truncate if necessary
        if input_ids.len() > self.max_length {
            input_ids.truncate(self.max_length);
            attention_mask.truncate(self.max_length);
        }

        // Pad to max_length
        while input_ids.len() < self.max_length {
            input_ids.push(0); // PAD token
            attention_mask.push(0);
        }

        Ok(TokenizedInput {
            input_ids,
            attention_mask,
            token_type_ids: vec![0; self.max_length],
        })
    }

    /// Tokenizes a pair of texts (for cross-encoder reranking).
    pub fn tokenize_pair(
        &self,
        text_a: &str,
        text_b: &str,
    ) -> Result<TokenizedInput, DatabaseError> {
        let encoding = self
            .tokenizer
            .encode((text_a, text_b), true)
            .map_err(|e| DatabaseError::IoError(format!("Failed to tokenize text pair: {}", e)))?;

        let mut input_ids = encoding.get_ids().to_vec();
        let mut attention_mask = encoding.get_attention_mask().to_vec();
        let mut token_type_ids = encoding.get_type_ids().to_vec();

        // Truncate if necessary
        if input_ids.len() > self.max_length {
            input_ids.truncate(self.max_length);
            attention_mask.truncate(self.max_length);
            token_type_ids.truncate(self.max_length);
        }

        // Pad to max_length
        while input_ids.len() < self.max_length {
            input_ids.push(0); // PAD token
            attention_mask.push(0);
            token_type_ids.push(0);
        }

        Ok(TokenizedInput {
            input_ids,
            attention_mask,
            token_type_ids,
        })
    }

    /// Tokenizes multiple texts in batch.
    pub fn tokenize_batch(&self, texts: &[&str]) -> Result<Vec<TokenizedInput>, DatabaseError> {
        texts.iter().map(|text| self.tokenize(text)).collect()
    }

    /// Tokenizes multiple text pairs in batch.
    pub fn tokenize_pairs_batch(
        &self,
        pairs: &[(&str, &str)],
    ) -> Result<Vec<TokenizedInput>, DatabaseError> {
        pairs
            .iter()
            .map(|(text_a, text_b)| self.tokenize_pair(text_a, text_b))
            .collect()
    }
}

/// Tokenized input ready for model inference.
#[derive(Debug, Clone)]
pub struct TokenizedInput {
    pub input_ids: Vec<u32>,
    pub attention_mask: Vec<u32>,
    pub token_type_ids: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn tokenized_input_has_correct_length() {
        // This test requires a real tokenizer file, so we skip it in CI
        if PathBuf::from("test_tokenizer.json").exists() {
            let tokenizer =
                BertTokenizer::from_file(&PathBuf::from("test_tokenizer.json"), 128).unwrap();

            let result = tokenizer.tokenize("Hello world").unwrap();
            assert_eq!(result.input_ids.len(), 128);
            assert_eq!(result.attention_mask.len(), 128);
            assert_eq!(result.token_type_ids.len(), 128);
        }
    }
}
