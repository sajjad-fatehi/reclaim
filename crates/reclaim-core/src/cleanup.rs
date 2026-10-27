//! Operation records are observations, never executable requests or replay tokens.
use crate::recognition::{Artifact, ChangeState, Nature};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_HISTORY: usize = 50;
pub const MAX_ITEMS: usize = 100;

pub fn filesystem_unavailability(artifact: Option<&Artifact>) -> &'static str {
    let Some(artifact) = artifact else {
        return "Unknown artifact. Source, personal data and arbitrary paths are protected.";
    };
    if let Some(worktree) = &artifact.worktree {
        if worktree.main {
            return "Main repositories are protected. Only linked worktrees could be considered.";
        }
        if worktree.locked {
            return "Worktree is locked. Removal unavailable; no force is permitted.";
        }
        if worktree.changes != ChangeState::Clean {
            return "Worktree has tracked, untracked, ignored or unknown changes. Removal blocked.";
        }
        return "Git removal unavailable: concurrent registry and data changes are not protected by a proven mutation boundary.";
    }
    if !artifact.complete {
        return "Recognition incomplete. Safety and ownership cannot be established.";
    }
    if !matches!(artifact.nature, Nature::Dependencies | Nature::Build) || !artifact.regenerable {
        return "Protected or unsupported artifact. Recognition, age and size do not authorize removal.";
    }
    "Trash unavailable: no proven operation binds removal to the reviewed identity and scope under concurrent replacement. No permanent-delete fallback."
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Trash,
    GitWorktree,
    DockerImage,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pending,
    Removed,
    Skipped,
    Failed,
    Changed,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemOutcome {
    pub label: String,
    pub method: Method,
    pub outcome: Outcome,
    pub explanation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub id: String,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    pub items: Vec<ItemOutcome>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct History {
    pub operations: Vec<Operation>,
}

fn valid_text(text: &str, maximum: usize) -> bool {
    !text.is_empty()
        && text.len() <= maximum
        && !text.chars().any(|character| {
            character.is_control()
                || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

impl History {
    pub fn validate(&self) -> bool {
        let mut identities = HashSet::new();
        self.operations.len() <= MAX_HISTORY
            && self.operations.iter().all(|operation| {
                valid_text(&operation.id, 64)
                    && identities.insert(&operation.id)
                    && operation.started_at_ms > 0
                    && operation
                        .finished_at_ms
                        .is_none_or(|finished| finished >= operation.started_at_ms)
                    && !operation.items.is_empty()
                    && operation.items.len() <= MAX_ITEMS
                    && operation.items.iter().all(|item| {
                        valid_text(&item.label, 4096)
                            && valid_text(&item.explanation, 512)
                            && (operation.finished_at_ms.is_none()
                                || item.outcome != Outcome::Pending)
                    })
            })
    }

    /// Restart cannot determine whether a process died before or after mutation.
    pub fn mark_interrupted(&mut self) {
        for operation in &mut self.operations {
            for item in &mut operation.items {
                if item.outcome == Outcome::Pending {
                    item.outcome = Outcome::Unknown;
                    item.explanation =
                        "Interrupted. Removal status unknown; inspect and rescan before a new review."
                            .into();
                }
            }
        }
    }

    pub fn record(&mut self, operation: Operation) -> Result<(), &'static str> {
        let mut updated = self.clone();
        updated.operations.retain(|entry| entry.id != operation.id);
        updated.operations.insert(0, operation);
        updated.operations.truncate(MAX_HISTORY);
        if !updated.validate() {
            return Err("Invalid operation history");
        }
        *self = updated;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(identity: usize) -> Operation {
        Operation {
            id: format!("fixture-{identity}"),
            started_at_ms: 1,
            finished_at_ms: None,
            items: vec![ItemOutcome {
                label: "SYNTHETIC target".into(),
                method: Method::Trash,
                outcome: Outcome::Pending,
                explanation: "Not yet completed".into(),
            }],
        }
    }

    #[test]
    fn bounds_records_and_preserves_completed_outcomes_after_interruption() {
        let mut history = History::default();
        for identity in 0..MAX_HISTORY + 10 {
            history.record(operation(identity)).unwrap();
        }
        assert_eq!(history.operations.len(), MAX_HISTORY);
        assert_eq!(history.operations[0].id, "fixture-59");
        history.operations[0].items[0].outcome = Outcome::Removed;
        history.operations[1].items[0].outcome = Outcome::Failed;
        history.mark_interrupted();
        assert_eq!(history.operations[0].items[0].outcome, Outcome::Removed);
        assert_eq!(history.operations[1].items[0].outcome, Outcome::Failed);
        assert_eq!(history.operations[2].items[0].outcome, Outcome::Unknown);
        assert!(history.validate());
        let serialized = serde_json::to_string(&history).unwrap();
        assert!(!serialized.contains("pending"));
    }

    #[test]
    fn rejects_invalid_records_without_changing_history() {
        let mut history = History::default();
        history.record(operation(1)).unwrap();
        let mut invalid = operation(2);
        invalid.items[0].label = "secret\nmisleading".into();
        assert!(history.record(invalid).is_err());
        assert_eq!(history.operations.len(), 1);
        let mut invalid = operation(2);
        invalid.finished_at_ms = Some(2);
        assert!(history.record(invalid).is_err());
        let mut invalid = operation(2);
        invalid.items = vec![invalid.items[0].clone(); MAX_ITEMS + 1];
        assert!(history.record(invalid).is_err());
        let mut duplicate = history.clone();
        duplicate.operations.push(operation(1));
        assert!(!duplicate.validate());
        assert!(serde_json::from_str::<History>(r#"{"operations":[],"replay":true}"#).is_err());
    }
}
