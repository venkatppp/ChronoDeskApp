//! ML Layer (blueprint §4.2, §6).
//!
//! Hosts on-device ONNX Runtime inference for workspace boundary
//! clustering, file classification, duplicate detection, and embedding
//! generation. Every model here is small enough to run in real time on a
//! laptop CPU; nothing in this module ever makes a network call.
//!
//! **Status:** scaffold only. Implemented in Phase 5 (Machine Learning Layer).
