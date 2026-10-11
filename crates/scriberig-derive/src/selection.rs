//! Selects immutable result records using caller-supplied policy.

use crate::freshness::{FreshnessPolicy, FreshnessState, evaluate_freshness};
use crate::profile::DerivationProfile;
use crate::result::{DerivedResultRecord, DerivedResultStatus};

/// Selects a compatible candidate and defines how failures/freshness are handled.
///
/// Candidate slice order is the caller's explicit ranking; selection never
/// mutates records or persists a preferred-result pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionPolicy {
    /// Required exact profile; `None` accepts any profile.
    pub profile: Option<DerivationProfile>,
    /// Include deterministic failed results for inspection.
    pub include_deterministic_failures: bool,
    /// Require freshness at the provided evaluation time.
    pub require_fresh: bool,
    /// Explicit logical timestamp used for freshness evaluation.
    pub now: u64,
}

/// Explains which candidate was chosen or why no candidate qualified.
#[derive(Clone, Debug, PartialEq)]
pub enum SelectionOutcome<'a> {
    /// Candidate was selected according to the requested policy.
    Selected {
        /// Exact immutable record selected by candidate ranking.
        record: &'a DerivedResultRecord,
    },
    /// No candidate met profile, status, and freshness requirements.
    NoMatch {
        /// Structured reason no candidate qualified.
        reason: SelectionReason,
    },
}

/// Provides a structured reason for a selection result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SelectionReason {
    /// Candidate matched exact profile and status requirements.
    ExactProfile,
    /// No candidate had the required profile.
    ProfileMismatch,
    /// Candidates existed but none had a reusable status.
    StatusNotReusable,
    /// Candidates existed but none met freshness requirements.
    FreshnessExpired,
    /// No candidates were supplied.
    NoStoredResult,
}

/// Selects the first eligible immutable candidate without mutating it.
///
/// # Parameters
/// - `candidates`: Existing records in caller-defined ranking order.
/// - `policy`: Profile, failure, and freshness constraints.
/// - `freshness`: Freshness policy applied to observations.
///
/// # Returns
/// The selected exact record or a structured no-match explanation.
#[must_use]
pub fn select<'a>(
    candidates: &'a [DerivedResultRecord],
    policy: &SelectionPolicy,
    freshness: &FreshnessPolicy,
) -> SelectionOutcome<'a> {
    if candidates.is_empty() {
        return SelectionOutcome::NoMatch {
            reason: SelectionReason::NoStoredResult,
        };
    }
    let mut profile_seen = false;
    let mut reusable_seen = false;
    for record in candidates {
        if policy
            .profile
            .as_ref()
            .is_some_and(|profile| profile != &record.key().profile)
        {
            continue;
        }
        profile_seen = true;
        let reusable = matches!(record.status(), DerivedResultStatus::Succeeded)
            || (policy.include_deterministic_failures
                && matches!(record.status(), DerivedResultStatus::FailedDeterministic));
        if !reusable {
            continue;
        }
        reusable_seen = true;
        if policy.require_fresh
            && !matches!(
                evaluate_freshness(freshness, record.freshness(), policy.now),
                FreshnessState::Fresh | FreshnessState::NotApplicable
            )
        {
            continue;
        }
        return SelectionOutcome::Selected { record };
    }
    let reason = if !profile_seen {
        SelectionReason::ProfileMismatch
    } else if !reusable_seen {
        SelectionReason::StatusNotReusable
    } else {
        SelectionReason::FreshnessExpired
    };
    SelectionOutcome::NoMatch { reason }
}
