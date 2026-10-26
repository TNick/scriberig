//! Verifies active branch slots enforce and release the configured budget.

use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_agent_runtime::orchestration::RuntimeLimits;
use scriberig_core::EntityId;

use super::ActiveBranchGuard;

/// Rejects branch admission at capacity and admits work after a slot is released.
#[test]
fn active_branch_guard_enforces_limit_and_releases_slots() {
    let scope = ChildExecutionScope {
        session: EntityId::new(),
        parent_branch: EntityId::new(),
        depth: 0,
        active_branches: std::sync::Arc::default(),
        semantic_scope: None,
        provider_resolver: None,
    };
    let limit = RuntimeLimits::default().max_active_branches;
    let mut guards = (0..limit)
        .map(|_| ActiveBranchGuard::acquire(&scope).expect("available active slot"))
        .collect::<Vec<_>>();

    assert!(ActiveBranchGuard::acquire(&scope).is_none());
    drop(guards.pop());
    assert!(ActiveBranchGuard::acquire(&scope).is_some());
    drop(guards);
    assert_eq!(
        scope
            .active_branches
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
}
