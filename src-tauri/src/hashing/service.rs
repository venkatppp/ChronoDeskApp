//! SHA-256 hashing service with streaming I/O.

use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during file hashing.
#[derive(Debug, Error)]
pub enum HashingError {
    /// The file could not be opened or read.
    #[error("failed to read file: {0}")]
    Io(#[from] io::Error),

    /// The file was deleted or became inaccessible during hashing.
    #[error("file became inaccessible during hashing")]
    FileDisappeared,
}

/// Service for computing SHA-256 content hashes.
///
/// Uses buffered streaming I/O to hash files of any size without loading
/// entire contents into memory. Safe for multi-gigabyte files.
#[derive(Debug, Clone)]
pub struct HashingService {
    /// Buffer size for streaming reads (64KB).
    buffer_size: usize,
}

impl Default for HashingService {
    fn default() -> Self {
        Self::new()
    }
}

impl HashingService {
    /// Creates a new hashing service with default buffer size (64KB).
    pub fn new() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB buffer
        }
    }

    /// Creates a hashing service with a custom buffer size.
    ///
    /// Useful for testing or tuning performance based on file size patterns.
    pub fn with_buffer_size(buffer_size: usize) -> Self {
        Self { buffer_size }
    }

    /// Computes the SHA-256 hash of a file at the given path.
    ///
    /// Uses buffered streaming I/O, so memory usage is constant regardless
    /// of file size.
    ///
    /// # Errors
    /// - [`HashingError::Io`] if the file cannot be opened or read
    /// - [`HashingError::FileDisappeared`] if the file is deleted during read
    ///
    /// # Examples
    /// ```no_run
    /// use chronodesk_lib::hashing::HashingService;
    ///
    /// let service = HashingService::new();
    /// let hash = service.hash_file("/path/to/file.txt")?;
    /// assert_eq!(hash.len(), 64); // SHA-256 hex string
    /// # Ok::<(), chronodesk_lib::hashing::HashingError>(())
    /// ```
    pub fn hash_file(&self, path: impl AsRef<Path>) -> Result<String, HashingError> {
        let file = std::fs::File::open(path.as_ref())?;
        let reader = std::io::BufReader::with_capacity(self.buffer_size, file);
        self.hash_reader(reader)
    }

    /// Computes the SHA-256 hash of data from any reader.
    ///
    /// Allows hashing from in-memory buffers, network streams, or any
    /// [`std::io::Read`] source, not just files.
    ///
    /// # Errors
    /// - [`HashingError::Io`] if reading fails
    pub fn hash_reader(&self, mut reader: impl Read) -> Result<String, HashingError> {
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; self.buffer_size];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => hasher.update(&buffer[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(HashingError::Io(e)),
            }
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn hash_empty_input() {
        let service = HashingService::new();
        let reader = Cursor::new(b"");
        let hash = service.hash_reader(reader).unwrap();

        // Known SHA-256 of empty string
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hash_known_vector_short() {
        let service = HashingService::new();
        let reader = Cursor::new(b"abc");
        let hash = service.hash_reader(reader).unwrap();

        // Known SHA-256 of "abc"
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_known_vector_long() {
        let service = HashingService::new();
        let input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        let reader = Cursor::new(input);
        let hash = service.hash_reader(reader).unwrap();

        // Known SHA-256 of the input above
        assert_eq!(
            hash,
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn hash_large_input_streaming() {
        // Test that large inputs work with small buffer (verifies streaming)
        let service = HashingService::with_buffer_size(16); // Tiny buffer
        let input = vec![b'x'; 1024 * 1024]; // 1MB of 'x'
        let reader = Cursor::new(&input);
        let hash = service.hash_reader(reader).unwrap();

        // Verify it produces consistent result
        assert_eq!(hash.len(), 64);

        // Hash again with default buffer size, should match
        let service2 = HashingService::new();
        let reader2 = Cursor::new(&input);
        let hash2 = service2.hash_reader(reader2).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn hash_identical_content_produces_same_hash() {
        let service = HashingService::new();

        let hash1 = service.hash_reader(Cursor::new(b"test content")).unwrap();
        let hash2 = service.hash_reader(Cursor::new(b"test content")).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_different_content_produces_different_hash() {
        let service = HashingService::new();

        let hash1 = service.hash_reader(Cursor::new(b"content A")).unwrap();
        let hash2 = service.hash_reader(Cursor::new(b"content B")).unwrap();

        assert_ne!(hash1, hash2);
    }
}
