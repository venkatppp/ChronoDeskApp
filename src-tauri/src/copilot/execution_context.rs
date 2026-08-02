//! Execution context - variable binding for a running plan execution.
//!
//! Introduced in RC-5 M3. Exists only while an execution is active (kept in
//! `ExecutionEngine::active_executions`), stores the structured output of each
//! completed step plus shared variables, and resolves `{{...}}` template
//! references before a downstream tool is invoked. In-memory only; no schema.

use std::collections::HashMap;

use serde_json::{json, Value};
use uuid::Uuid;

/// Prefix used when a variable cannot be resolved, so the planner can map the
/// engine-side step failure back to a structured error.
pub const UNRESOLVED_VARIABLE_MARKER: &str = "unresolved variable";

/// Errors produced while resolving `{{...}}` references.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContextError {
    #[error("unresolved variable: {0}")]
    Unresolved(String),

    #[error("unresolved variable: step '{0}' has no stored output")]
    MissingStep(String),

    #[error("unresolved variable: missing field '{field}' in '{template}'")]
    MissingField { template: String, field: String },

    #[error("invalid template: {0}")]
    Malformed(String),
}

impl ContextError {
    /// True when this error describes an unresolvable variable reference
    /// (as opposed to a malformed template), used for message classification.
    pub fn is_unresolved(&self) -> bool {
        matches!(
            self,
            ContextError::Unresolved(_)
                | ContextError::MissingStep(_)
                | ContextError::MissingField { .. }
        )
    }
}

/// One completed step's stored output.
#[derive(Debug, Clone)]
struct StepOutput {
    output: Value,
}

/// Execution-scoped variable binding store.
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    /// Shared variables: always carry `goal` and `workspace` (when the
    /// execution has one); caller-scoped variables via [`Self::set_variable`].
    variables: HashMap<String, Value>,
    /// Tool outputs keyed by step name (tool name) and step index so later
    /// steps can consume them through `{{steps.<name>.<path>}}` templates.
    steps: HashMap<String, StepOutput>,
}

impl ExecutionContext {
    /// Creates an empty context carrying the goal text and (optionally) the
    /// workspace the execution is bound to.
    pub fn new(workspace_id: Option<Uuid>, goal: String) -> Self {
        let mut variables = HashMap::new();
        variables.insert("goal".to_string(), Value::String(goal));
        if let Some(id) = workspace_id {
            variables.insert(
                "workspace".to_string(),
                json!({ "id": id.to_string(), "workspace_id": id.to_string() }),
            );
        }
        Self {
            variables,
            steps: HashMap::new(),
        }
    }

    /// Sets (or overwrites) a named execution variable.
    pub fn set_variable(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// Records a completed step's tool output so downstream steps can resolve
    /// `{{steps.<name>.<path>}}`. Indexed access is also stored.
    pub fn set_step_output(&mut self, step: usize, name: Option<&str>, output: Value) {
        if let Some(name) = name {
            self.steps.insert(
                name.to_string(),
                StepOutput {
                    output: output.clone(),
                },
            );
        }
        self.steps.insert(step.to_string(), StepOutput { output });
    }

    /// Stores a step output under an opaque key (used by non-plan steps).
    pub fn save_step_output(&mut self, key: String, output: Value) {
        self.steps.insert(key, StepOutput { output });
    }

    /// Whether any output has been stored for `name`.
    pub fn has_step(&self, name: &str) -> bool {
        self.steps.contains_key(name)
    }

    /// Returns the stored output for a named step, if any.
    pub fn step_output(&self, name: &str) -> Option<&Value> {
        self.steps.get(name).map(|s| &s.output)
    }

    /// Returns a snapshot of the stored step outputs (name/step-number → JSON).
    pub fn snapshot(&self) -> Vec<(String, Value)> {
        self.steps
            .iter()
            .map(|(k, v)| (k.clone(), v.output.clone()))
            .collect()
    }

    /// Resolves every template reference in a JSON value, returning a copy
    /// with substituted values. Strings without `{{...}}` pass through.
    pub fn resolve(&self, value: &Value) -> Result<Value, ContextError> {
        match value {
            Value::String(s) => self.resolve_string(s),
            Value::Array(items) => items
                .iter()
                .map(|v| self.resolve(v))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (key, value) in map {
                    out.insert(key.clone(), self.resolve(value)?);
                }
                Ok(Value::Object(out))
            }
            other => Ok(other.clone()),
        }
    }

    /// Resolves a single string, which may contain zero, one, or many
    /// `{{...}}` references.
    pub fn resolve_string(&self, input: &str) -> Result<Value, ContextError> {
        let templates = extract_templates(input);
        if templates.is_empty() {
            return Ok(Value::String(input.to_string()));
        }
        if templates.len() == 1 && is_whole_template(input) {
            return self.resolve_template(&templates[0]);
        }
        let mut out = input.to_string();
        for template in templates {
            let value = self.resolve_template(&template)?;
            let text = to_text(&value);
            let needle = format!("{{{{{}}}}}", template);
            out = out.replacen(&needle, &text, 1);
        }
        Ok(Value::String(out))
    }

    /// Resolves the body of a single `{{...}}` reference (without braces).
    pub fn resolve_template(&self, expr: &str) -> Result<Value, ContextError> {
        let expr = expr.trim();
        if expr.is_empty() {
            return Err(ContextError::Malformed("empty template".to_string()));
        }

        let parts = split_path(expr);

        match parts.first().map(|s| s.as_str()) {
            Some("steps") => {
                let Some((name, path)) =
                    parts.get(1).map(|name| (name.clone(), parts[2..].to_vec()))
                else {
                    return Err(ContextError::Malformed(expr.to_string()));
                };
                let output = self
                    .steps
                    .get(&name)
                    .ok_or_else(|| ContextError::MissingStep(name.clone()))?
                    .output
                    .clone();
                lookup(&output, &path, expr)
            }
            Some(root) => {
                let base = self
                    .variables
                    .get(root)
                    .cloned()
                    .ok_or_else(|| ContextError::Unresolved(expr.to_string()))?;
                lookup(&base, &parts[1..], expr)
            }
            None => Err(ContextError::Malformed(expr.to_string())),
        }
    }
}

/// Splits a dotted path into individual lookups, expanding array indexes like
/// `results[0]` into `results` + `[0]`, so `lookup` can walk them in order.
fn split_path(expr: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in expr.split('.') {
        let mut rest = part;
        while let Some(idx) = rest.find('[') {
            if idx > 0 {
                out.push(rest[..idx].to_string());
            }
            if let Some(close) = rest[idx..].find(']') {
                out.push(format!("[{}]", &rest[idx + 1..idx + close]));
                rest = &rest[idx + close + 1..];
            } else {
                out.push(rest[idx..].to_string());
                rest = "";
            }
        }
        if !rest.is_empty() {
            out.push(rest.to_string());
        }
    }
    out
}

/// Looks up a dotted path of field segments against a JSON value.
fn lookup(base: &Value, segments: &[String], template: &str) -> Result<Value, ContextError> {
    let mut current = base.clone();
    for segment in segments {
        match segment.as_str() {
            segment if segment.starts_with('[') && segment.ends_with(']') => {
                let index: usize = segment[1..segment.len() - 1].parse().map_err(|_| {
                    ContextError::MissingField {
                        template: template.to_string(),
                        field: segment.to_string(),
                    }
                })?;
                current =
                    current
                        .get(index)
                        .cloned()
                        .ok_or_else(|| ContextError::MissingField {
                            template: template.to_string(),
                            field: segment.to_string(),
                        })?;
            }
            field => {
                current =
                    current
                        .get(field)
                        .cloned()
                        .ok_or_else(|| ContextError::MissingField {
                            template: template.to_string(),
                            field: field.to_string(),
                        })?;
            }
        }
    }
    Ok(current)
}

/// Renders a bound JSON value as embeddable text: strings are used verbatim,
/// everything else is the JSON encoding.
fn to_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

/// Extracts `{{...}}` bodies from a string, in order.
fn extract_templates(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '{' && chars[i + 1] == '{' {
            if let Some((body, next)) = find_closing(&chars, i + 2) {
                out.push(body.trim().to_string());
                i = next;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Finds `}}` from `from`, returning (body chars, index just past closing).
fn find_closing(chars: &[char], from: usize) -> Option<(String, usize)> {
    let mut i = from;
    while i + 1 < chars.len() {
        if chars[i] == '}' && chars[i + 1] == '}' {
            let body: String = chars[from..i].iter().collect();
            return Some((body, i + 2));
        }
        i += 1;
    }
    None
}

/// True when `input` is exactly one `{{...}}` template with no surrounding text.
fn is_whole_template(input: &str) -> bool {
    let templates = extract_templates(input);
    if templates.len() != 1 {
        return false;
    }
    let body = &templates[0];
    let candidates = [format!("{{{{{}}}}}", body), format!("{{{{ {} }}}}", body)];
    candidates.iter().any(|c| c == input.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ExecutionContext {
        ExecutionContext::new(Some(uuid::Uuid::new_v4()), "resume focus".to_string())
    }

    #[test]
    fn stores_and_reads_step_outputs() {
        let mut ctx = context();
        ctx.set_step_output(
            0,
            Some("list_workspaces"),
            json!({
                "workspaces": [{ "id": "w1", "path": "/one" }],
                "active": { "id": "w1" }
            }),
        );
        assert!(ctx.has_step("list_workspaces"));
        let output = ctx.step_output("list_workspaces").unwrap();
        assert_eq!(output["active"]["id"], "w1");
        // Also accessible by index.
        assert!(ctx.has_step("0"));
    }

    #[test]
    fn variable_substitution_returns_scalar() {
        let ctx = context();
        let resolved = ctx.resolve_string("{{goal}}").expect("goal resolves");
        assert_eq!(resolved, Value::String("resume focus".to_string()));

        let ws = ctx
            .resolve_string("{{workspace.id}}")
            .expect("workspace resolves");
        assert!(ws.as_str().is_some() && !ws.as_str().unwrap().is_empty());
    }

    #[test]
    fn nested_json_lookup_resolves() {
        let mut ctx = context();
        ctx.set_step_output(
            0,
            Some("search"),
            json!({ "results": [{ "path": "/src/main.rs" }] }),
        );
        let resolved = ctx
            .resolve_string("{{steps.search.results[0].path}}")
            .expect("nested lookup resolves");
        assert_eq!(resolved, Value::String("/src/main.rs".to_string()));
    }

    #[test]
    fn array_index_into_step_output() {
        let mut ctx = context();
        ctx.set_step_output(
            0,
            Some("list_workspaces"),
            json!({ "workspaces": [{ "id": "w1" }] }),
        );
        let resolved = ctx
            .resolve_string("{{steps.list_workspaces.workspaces[0].id}}")
            .expect("array index resolves");
        assert_eq!(resolved, Value::String("w1".to_string()));
    }

    #[test]
    fn missing_variable_errors_are_structured() {
        let ctx = context();
        let error = ctx
            .resolve_string("{{steps.never_ran.output}}")
            .unwrap_err();
        assert!(error.is_unresolved());
        assert!(matches!(error, ContextError::MissingStep(_)));

        let error = ctx.resolve_string("{{unknown.path}}").unwrap_err();
        assert!(error.is_unresolved());

        let error = ctx
            .resolve_string("{{steps.search.missing.deep}}")
            .unwrap_err();
        assert!(error.is_unresolved());
    }

    #[test]
    fn embedded_template_substitutes_text() {
        let ctx = context();
        let resolved = ctx
            .resolve_string("goal is {{goal}} today")
            .expect("embedded substitution");
        assert_eq!(
            resolved,
            Value::String("goal is resume focus today".to_string())
        );
    }

    #[test]
    fn resolves_whole_argument_objects() {
        let mut ctx = context();
        ctx.set_step_output(
            0,
            Some("list_workspaces"),
            json!({ "workspaces": [{ "id": "w1" }] }),
        );
        let args = json!({
            "workspace_id": "{{steps.list_workspaces.workspaces[0].id}}",
            "limit": 5
        });
        let resolved = ctx.resolve(&args).expect("args resolve");
        assert_eq!(resolved["workspace_id"], "w1");
        assert_eq!(resolved["limit"], 5);
    }
}
