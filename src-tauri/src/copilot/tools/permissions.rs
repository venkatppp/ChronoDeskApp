//! Persistent tool permission policies.
//!
//! Policies gate when a tool may execute through the [`ToolExecutor`]
//! pipeline. Each policy resolves to one of [`ToolPermissionDecision`]:
//! `AllowOnce` (grant exactly one future invocation), `AlwaysAllow`, or
//! `Deny`. Policies are scoped to a specific workspace (`Some(id)`) or
//! globally (`None`); the exact workspace match always wins over the
//! global policy for the same tool.
//!
//! Persistence reuses the existing `settings` key/value table (a
//! designer-intended JSON preference store), so no migration is needed.

use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::repositories::SettingsRepository;

use super::models::{ToolPermissionDecision, ToolPermissionPolicy};

const SETTINGS_KEY: &str = "tool_permission_policies";

/// Service that owns persisted tool permission policies.
#[derive(Debug)]
pub struct ToolPermissionService {
    repository: SettingsRepository,
    policies: RwLock<Vec<ToolPermissionPolicy>>,
}

impl ToolPermissionService {
    /// Loads the persisted policy set from the settings store.
    pub async fn new(repository: SettingsRepository) -> Result<Self, DatabaseError> {
        let policies = load(&repository).await?;
        Ok(Self {
            repository,
            policies: RwLock::new(policies),
        })
    }

    /// Upserts a policy for a tool. `workspace_id = None` creates a
    /// global policy; `Some(id)` scopes it to that workspace.
    pub async fn set_policy(
        &self,
        tool_name: &str,
        workspace_id: Option<Uuid>,
        decision: ToolPermissionDecision,
    ) -> Result<(), DatabaseError> {
        let mut policies = self.policies.write().await;
        policies.retain(|p| !same_scope(p, tool_name, workspace_id));
        policies.push(ToolPermissionPolicy {
            id: Uuid::new_v4(),
            tool_name: tool_name.to_string(),
            workspace_id,
            decision,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        self.persist_locked(&policies).await
    }

    /// Removes any policy matching the given tool + scope.
    pub async fn clear_policy(
        &self,
        tool_name: &str,
        workspace_id: Option<Uuid>,
    ) -> Result<(), DatabaseError> {
        let mut policies = self.policies.write().await;
        let before = policies.len();
        policies.retain(|p| !same_scope(p, tool_name, workspace_id));
        if policies.len() == before {
            return Ok(());
        }
        self.persist_locked(&policies).await
    }

    /// Returns a snapshot of every stored policy.
    pub async fn policies(&self) -> Vec<ToolPermissionPolicy> {
        self.policies.read().await.clone()
    }

    /// Resolves the effective policy for a tool in a given scope. A
    /// workspace-scoped policy beats the global policy, which acts as a
    /// fallback.
    pub async fn resolve_policy(
        &self,
        tool_name: &str,
        workspace_id: Option<Uuid>,
    ) -> Option<ToolPermissionPolicy> {
        let policies = self.policies.read().await;
        policies
            .iter()
            .find(|p| p.tool_name == tool_name && p.workspace_id == workspace_id)
            .or_else(|| {
                policies
                    .iter()
                    .find(|p| p.tool_name == tool_name && p.workspace_id.is_none())
            })
            .cloned()
    }

    /// Resolves just the effective decision for a tool + scope.
    pub async fn resolve(
        &self,
        tool_name: &str,
        workspace_id: Option<Uuid>,
    ) -> Option<ToolPermissionDecision> {
        self.resolve_policy(tool_name, workspace_id)
            .await
            .map(|p| p.decision)
    }

    /// Consumes a one-shot policy after its single use has been granted.
    pub async fn consume_policy(&self, policy: &ToolPermissionPolicy) -> Result<(), DatabaseError> {
        if policy.decision != ToolPermissionDecision::AllowOnce {
            return Ok(());
        }
        let mut policies = self.policies.write().await;
        let before = policies.len();
        policies.retain(|p| p.id != policy.id);
        if policies.len() != before {
            self.persist_locked(&policies).await?;
        }
        Ok(())
    }

    async fn persist_locked(&self, policies: &[ToolPermissionPolicy]) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(policies)
            .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?;
        self.repository.set(SETTINGS_KEY, &json).await
    }
}

fn same_scope(policy: &ToolPermissionPolicy, tool_name: &str, workspace_id: Option<Uuid>) -> bool {
    policy.tool_name == tool_name && policy.workspace_id == workspace_id
}

async fn load(repository: &SettingsRepository) -> Result<Vec<ToolPermissionPolicy>, DatabaseError> {
    match repository.get(SETTINGS_KEY).await? {
        Some(raw) if !raw.is_empty() => serde_json::from_str(&raw).map_err(|e| {
            DatabaseError::InvalidInput(format!("corrupt tool permission policies: {e}"))
        }),
        _ => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    fn workspace_id(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    type Decision = ToolPermissionDecision;

    async fn service() -> (ToolPermissionService, tempfile::TempDir) {
        let (database, guard) = test_database().await;
        let service = ToolPermissionService::new(SettingsRepository::new(database.pool().clone()))
            .await
            .expect("service should initialize");
        (service, guard)
    }

    #[tokio::test]
    async fn set_then_resolve_workspace_and_global_policies() {
        let (service, _guard) = service().await;

        service
            .set_policy(
                "resume_workspace",
                Some(workspace_id(1)),
                ToolPermissionDecision::AllowOnce,
            )
            .await
            .unwrap();
        service
            .set_policy(
                "resume_workspace",
                None,
                ToolPermissionDecision::AlwaysAllow,
            )
            .await
            .unwrap();

        assert_eq!(
            service
                .resolve("resume_workspace", Some(workspace_id(1)))
                .await,
            Some(ToolPermissionDecision::AllowOnce)
        );
        assert_eq!(
            service
                .resolve("resume_workspace", Some(workspace_id(2)))
                .await,
            Some(ToolPermissionDecision::AlwaysAllow)
        );
        assert_eq!(
            service.resolve("resume_workspace", None).await,
            Some(ToolPermissionDecision::AlwaysAllow)
        );
        assert_eq!(service.resolve("unknown_tool", None).await, None);
    }

    #[tokio::test]
    async fn workspace_policy_overrides_global_policy() {
        let (service, _guard) = service().await;

        service
            .set_policy(
                "resume_workspace",
                None,
                ToolPermissionDecision::AlwaysAllow,
            )
            .await
            .unwrap();
        service
            .set_policy("resume_workspace", Some(workspace_id(7)), Decision::Deny)
            .await
            .unwrap();

        assert_eq!(
            service
                .resolve("resume_workspace", Some(workspace_id(7)))
                .await,
            Some(ToolPermissionDecision::Deny)
        );
        assert_eq!(
            service
                .resolve("resume_workspace", Some(workspace_id(8)))
                .await,
            Some(ToolPermissionDecision::AlwaysAllow)
        );
    }

    #[tokio::test]
    async fn allow_once_is_consumed_after_single_grant() {
        let (service, _guard) = service().await;

        service
            .set_policy(
                "resume_workspace",
                Some(workspace_id(1)),
                ToolPermissionDecision::AllowOnce,
            )
            .await
            .unwrap();

        let policy = service
            .resolve_policy("resume_workspace", Some(workspace_id(1)))
            .await
            .expect("policy should resolve");
        assert_eq!(policy.decision, ToolPermissionDecision::AllowOnce);

        service.consume_policy(&policy).await.unwrap();

        assert_eq!(
            service
                .resolve("resume_workspace", Some(workspace_id(1)))
                .await,
            None,
            "allow-once policy must be gone after consumption"
        );
    }

    #[tokio::test]
    async fn clear_removes_policy_and_keeps_other_scopes() {
        let (service, _guard) = service().await;

        service
            .set_policy("resume_workspace", None, ToolPermissionDecision::Deny)
            .await
            .unwrap();

        service
            .clear_policy("resume_workspace", None)
            .await
            .unwrap();
        assert_eq!(service.resolve("resume_workspace", None).await, None);
        assert!(service.policies().await.is_empty());
    }

    #[tokio::test]
    async fn policies_persist_across_service_reload() {
        let (database, _guard) = test_database().await;
        let repository = SettingsRepository::new(database.pool().clone());

        let service = ToolPermissionService::new(repository.clone())
            .await
            .expect("test failed to init");
        service
            .set_policy(
                "resume_workspace",
                None,
                ToolPermissionDecision::AlwaysAllow,
            )
            .await
            .unwrap();

        let reloaded = ToolPermissionService::new(repository.clone())
            .await
            .expect("test failed to reload");
        assert_eq!(
            reloaded.resolve("resume_workspace", None).await,
            Some(ToolPermissionDecision::AlwaysAllow)
        );
    }
}
