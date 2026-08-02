# ChronoDesk RC-4 Engineering Report

**Release:** RC-4 (Release Candidate 4) — LLM-Native Tool Calling
**Commit:** 9708c18
**Date:** 2026-08-02
**Build Status:** ✅ All checks passing (228 unit + 5 integration + 1 doc-test)
**Previous Commit:** 279dff0 (RC-4 — Persistent Tool Permission System)

---

## Executive Summary

RC-4 delivers LLM-Native Tool Calling: the Copilot now advertises its tool
registry to the model, parses provider-native `tool_calls`, executes each one
through the existing `ToolExecutor`/permission pipeline, feeds structured
results back as `tool`-role messages, and iterates until the model returns a
plain answer or the iteration limit is reached.

**Impact:** +1,117 lines / -123 lines across 7 files, 0 breaking changes,
no schema migrations, 100% test suite green, all six validation gates pass.

---

## Architecture Changes

### Provider-Native Wire Support (`llm/models.rs`)

The LLM message model converged with the OpenAI function-calling wire format
so requests/responses round-trip through any compatible provider.

**New types:**
- `LLMToolCall` — `{ id, name, arguments }` with custom `Serialize`
  (emits `{ id, "type":"function", "function":{ name, arguments } }` with
  arguments as a JSON string) and custom `Deserialize` (accepts both the
  nested `function` shape and the flat shape).
- `LLMTool` / `LLMToolParameters` / `LLMToolParameter` /
  `LLMToolParameterType` — advertise tool schemas (JSON-schema style
  `{ type:"function", function:{ name, description, parameters } }`).
- `LLMMessage` gained `tool_calls: Option<Vec<LLMToolCall>>` and
  `tool_call_id: Option<String>` plus constructors `new()`,
  `tool_result()`, and `assistant_tool_calls()`.
- `LLMRequest` gained `tools: Option<Vec<LLMTool>>` (skipped when empty).
- `LLMResponse` gained `tool_calls: Option<Vec<LLMToolCall>>`.

### Provider Request / Response Parsing (`llm/openai_provider.rs`)

- `OpenAICompletionRequest` now carries `tools`.
- Non-streaming `complete()` parses `choice.message.tool_calls`.
- Streaming `complete_stream()` reconstructs tool calls from deltas: a
  `StreamAggregator` accumulates content, tool-call fragments
  (`index`/`id`/`function.name`/`function.arguments`), usage, and finish
  reason, producing a final `LLMResponse` on completion. Text chunks are
  still emitted live as `StreamEvent::Chunk`, so the frontend UX is unchanged.

### Tool Calling Loop (`copilot/tool_calling.rs` — NEW)

| Component | Responsibility |
|---|---|
| `ToolCallLoop` | Runs rounds: build `LLMRequest` → respond → 0-n tool calls → insert assistant tool-call message → one `tool` feedback message per call → repeat until plain answer or `max_iterations` |
| `ToolCallResponder` (trait) | Pluggable model I/O (streaming + non-streaming impls in `engine.rs`) so the loop is driver-agnostic |
| `ToolCallLoopStatus` | `Completed` / `MaxIterationsReached` |
| `ToolCallLoopError` | `Cancelled` / `Responder` / `Execution` |
| `build_tool_schemas` | `ToolDefinition::parameters → LLMTool` mapping, reusing the existing registry metadata |
| `feedback_content` | Renders `ToolInvocationResult` (including errors) into `tool`-message content |

`DEFAULT_MAX_TOOL_ITERATIONS = 8`, overridable via `with_max_iterations`.

**Key isolation guarantee:** the loop does NOT duplicate execution or
permission logic. Each call is parsed into a `ToolInvocationRequest` and
executed through `ToolExecutor::invoke_tool_with_context`, which continues
to enforce static registry metadata plus the persistent
`ToolPermissionService` runtime policy. Failures — permission denials, tool
errors, timeouts, unknown tools — are captured in feedback content and sent
back to the model so it can observe and recover, rather than aborting the
round-trip.

### Engine Integration (`copilot/engine.rs`)

- `run_stream` and `generate_response` now drive the loop via
  `StreamingResponder` (emits chunks live + returns the final response) and
  `NonStreamingResponder`.
- `build_llm_request` attaches `build_tool_schemas(...available_tools())`.
- Streaming cancellation is propagation-aware: a cancelled token surfaces as
  `ToolCallLoopError::Cancelled` and cancels the stream cleanly.
- `Source` reference for tool-calling responses is reported as
  `tool-calling loop (N rounds)`.

### Minor Literal Adaptations

- `llm/token_counter.rs` and `llm/hardening/tests.rs` updated for the
  `LLMMessage::new` / extended `LLMResponse` shapes. No behavior change.

---

## Implementation Details

### Request/Response Round-Trip

1. `CopilotEngine` builds messages + tool schemas and calls the responder.
2. The provider serializes `tools` on the request.
3. Model returns `tool_calls` (non-streaming) or the stream aggregator
   reconstructs them from deltas.
4. `ToolCallLoop` emits an assistant message with the tool-calls.
5. Each call → `ToolInvocationRequest` → `ToolExecutor` (validation,
   static + runtime permissions, timeout/retry, progress events).
6. Feedback message appended as a `tool` role with the invocation payload.
7. Repeat until a plain answer or iteration cap; accumulate `executions`
   for diagnostics.

### Cancellation

`ToolCallLoop::run` checks the shared `CancellationToken` before each round
and passes it into each invocation, so a user cancel aborts both the model
round-trips and any in-flight tool execution.

### Concurrency & Safety

- Loop is fully `Send`-safe; used inside `tokio::spawn` from `run_stream`.
- One writer per stream: the loop owns message history for the round.
- `max_iterations` is clamped to `>= 1`.

---

## Validation

| Check | Result |
|---|---|
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ |
| `cargo build` | ✅ |
| `cargo test` | ✅ 228 unit + 5 integration + 1 doc-test |
| `cd frontend && npm run build` | ✅ |
| `cd frontend && npx tsc --noEmit` | ✅ |

### New Backend Tests (`copilot/tool_calling.rs`)

| Test | Verifies |
|---|---|
| `single_tool_call_executes_then_answers` | 1 execution, plain-answer stop, `Completed` |
| `multiple_sequential_tool_calls_execute_in_order` | interleaves IDs, 2 executions, SSS |
| `permission_denied_feeds_error_back_but_loop_continues` | runtime deny → `Failed` with "denied" feedback, loop continues |
| `tool_failure_does_not_abort_loop` | unknown workspace → `Failed`, loop continues |
| `iteration_limit_protection_stops_the_loop` | hits cap, `MaxIterationsReached`, no further rounds |

---

## Security Review

- **No new secrets, credentials, or .env exposure** — unchanged.
- Permission enforcement unchanged: static registry levels + persistent
  runtime policy both gate every tool call; loop adds no bypass path.
- Tool failure feedback is returned to the model but never executes
  arbitrary code — execution remains bounded to the registered registry.
- Iteration cap limits model-driven recursion; each invocation still runs
  under the existing timeout/retry policy.
- No schema / IPC / trait changes: surface area for regressions is minimal.

---

## Production Readiness

- Zero breaking changes; all previous `#[tauri::command]` handlers, all
  `LLMProvider` trait methods, and the DB schema are untouched.
- Deterministic termination under cancellation, exhaustion, failure, or
  plain-answer completion.
- Streaming client behavior unchanged (events identical); server now also
  returns aggregated tool calls in addition to the live text stream.
- Verified full CI-equivalent locally across all six gates above.

---

## Remaining RC-4 Work

- **Conversation persistence of tool calls**: `tool_calls`/`tool_result`
  messages are currently sent to the model within the live loop but are not
  yet enriched in persisted `Message` rows via `CopilotRepository` (the
  live response content is persisted, the intermediate tool rounds are
  transient). If full audit-ability/replay of tool rounds is desired, add
  a repository layer for tool invocation records.
- **Tool-rate / defensive schemas**: `build_tool_schemas` is exact; future
  providers may prefer `auto`/`tool` choice control — not required now.
- **Frontend tool-call rendering**: the transcript can render the final
  answer; surfacing intermediate tool steps in the UI is a UX follow-up.

---

## Final Acceptance

- ✅ No re-audit performed
- ✅ `ToolExecutor`/permission services reused — no duplicated invocation logic
- ✅ 5 focused backend tests added and passing
- ✅ fmt / clippy / build / test / frontend build / tsc all green
- ✅ Committed (`9708c18`) and pushed to `origin/main`