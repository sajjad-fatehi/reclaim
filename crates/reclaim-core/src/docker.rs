//! Docker metrics are observations, never additive filesystem contributions or removal authority.
use crate::recognition::{Artifact, CleanupSupport, Confidence, Evidence, Family, Nature};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    Unavailable,
    UnsupportedContext,
    UnsupportedVersion,
    UnsupportedEngine,
    DaemonUnavailable,
    Malformed,
    Limited,
    Timeout,
    Cancelled,
    Busy,
    AccessDenied,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Image,
    Container,
    BuildCache,
    Volume,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub artifact: Artifact,
    pub kind: ResourceKind,
    pub identity: String,
    pub references: Option<u64>,
    pub active: Option<bool>,
    pub persistent: bool,
    pub shared: Option<bool>,
    pub logical_bytes: Option<String>,
    pub shared_bytes: Option<String>,
    pub metric_basis: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub items: Vec<Item>,
    pub accounting: &'static str,
}

pub fn validate_version(bytes: &[u8]) -> Result<String, Failure> {
    let version: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| Failure::Malformed)?;
    let platform = version["Platform"]["Name"]
        .as_str()
        .ok_or(Failure::UnsupportedEngine)?;
    if !platform.starts_with("Docker Engine") && !platform.starts_with("Docker Desktop") {
        return Err(Failure::UnsupportedEngine);
    }
    let major = version["Version"]
        .as_str()
        .and_then(|value| value.split('.').next())
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or(Failure::Malformed)?;
    let api = version["ApiVersion"].as_str().ok_or(Failure::Malformed)?;
    let minor = api
        .strip_prefix("1.")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or(Failure::Malformed)?;
    if !(24..=29).contains(&major) || !(43..=52).contains(&minor) {
        return Err(Failure::UnsupportedVersion);
    }
    Ok(format!("1.{minor}"))
}

fn optional_number(value: &serde_json::Value) -> Result<Option<u64>, Failure> {
    if value.is_null() || value.as_i64() == Some(-1) {
        return Ok(None);
    }
    value.as_u64().map(Some).ok_or(Failure::Malformed)
}

fn identity_valid(identity: &str, kind: ResourceKind) -> bool {
    if kind == ResourceKind::Volume {
        return !identity.is_empty()
            && identity.len() <= 255
            && identity
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte));
    }
    let identity = identity.strip_prefix("sha256:").unwrap_or(identity);
    if kind == ResourceKind::BuildCache {
        return !identity.is_empty()
            && identity.len() <= 128
            && identity.bytes().all(|byte| byte.is_ascii_alphanumeric());
    }
    identity.len() == 64 && identity.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn parse_inventory(bytes: &[u8]) -> Result<Inventory, Failure> {
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(Failure::Limited);
    }
    let usage: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| Failure::Malformed)?;
    let groups = [
        (ResourceKind::Image, usage.get("Images")),
        (ResourceKind::Container, usage.get("Containers")),
        (ResourceKind::Volume, usage.get("Volumes")),
        (ResourceKind::BuildCache, usage.get("BuildCache")),
    ];
    let mut items = Vec::new();
    let mut identities = HashSet::new();
    for (kind, resources) in groups {
        let resources = resources.ok_or(Failure::Malformed)?;
        let empty = Vec::new();
        let resources = if resources.is_null() {
            &empty
        } else {
            resources.as_array().ok_or(Failure::Malformed)?
        };
        if items.len().saturating_add(resources.len()) > 4096 {
            return Err(Failure::Limited);
        }
        for resource in resources {
            let identity = resource[if kind == ResourceKind::Volume {
                "Name"
            } else if kind == ResourceKind::BuildCache {
                "ID"
            } else {
                "Id"
            }]
            .as_str()
            .ok_or(Failure::Malformed)?
            .to_owned();
            if !identity_valid(&identity, kind) {
                return Err(Failure::Malformed);
            }
            let finding_id = format!("docker:{kind:?}:{identity}");
            if !identities.insert(finding_id.clone()) {
                return Err(Failure::Malformed);
            }
            let (size, references, active, shared_bytes, shared) = match kind {
                ResourceKind::Image => (
                    optional_number(&resource["Size"])?,
                    optional_number(&resource["Containers"])?,
                    None,
                    optional_number(&resource["SharedSize"])?,
                    None,
                ),
                ResourceKind::Container => {
                    let state = resource["State"].as_str().ok_or(Failure::Malformed)?;
                    if ![
                        "created",
                        "running",
                        "paused",
                        "restarting",
                        "removing",
                        "exited",
                        "dead",
                    ]
                    .contains(&state)
                    {
                        return Err(Failure::Malformed);
                    }
                    (
                        optional_number(&resource["SizeRw"])?,
                        None,
                        Some(!["created", "exited", "dead"].contains(&state)),
                        None,
                        None,
                    )
                }
                ResourceKind::Volume => (
                    optional_number(&resource["UsageData"]["Size"])?,
                    optional_number(&resource["UsageData"]["RefCount"])?,
                    None,
                    None,
                    None,
                ),
                ResourceKind::BuildCache => (
                    optional_number(&resource["Size"])?,
                    None,
                    Some(resource["InUse"].as_bool().ok_or(Failure::Malformed)?),
                    None,
                    Some(resource["Shared"].as_bool().ok_or(Failure::Malformed)?),
                ),
            };
            if size
                .zip(shared_bytes)
                .is_some_and(|(size, shared)| shared > size)
            {
                return Err(Failure::Malformed);
            }
            let persistent = matches!(kind, ResourceKind::Volume | ResourceKind::Container);
            let consequence = if persistent {
                "May contain persistent user data. Preserve volumes and container writable layers; inspection only."
            } else {
                "Images and caches may be shared or referenced. Rebuilding can require downloads and unavailable inputs; inspection only."
            };
            items.push(Item {
                artifact: Artifact {
                    finding_id: finding_id.clone(),
                    family: Family::Docker,
                    nature: if persistent {
                        Nature::Protected
                    } else {
                        Nature::SharedStore
                    },
                    project_id: None,
                    display_name: finding_id.clone(),
                    parent_id: None,
                    kind: crate::scan::Kind::Unknown,
                    confidence: Confidence::Verified,
                    evidence: vec![Evidence {
                        finding_id,
                        method: "local_docker_engine_system_df".into(),
                    }],
                    consequence: consequence.into(),
                    cleanup_support: CleanupSupport::InspectionOnly,
                    regenerable: !persistent,
                    complete: true,
                    exclusive_logical_bytes: "0".into(),
                    exclusive_allocated_bytes: "0".into(),
                    owned_logical_bytes: None,
                    owned_allocated_bytes: None,
                    worktree: None,
                },
                kind,
                identity,
                references,
                active,
                persistent,
                shared,
                logical_bytes: size.map(|bytes| bytes.to_string()),
                shared_bytes: shared_bytes.map(|bytes| bytes.to_string()),
                metric_basis: "engine_reported_logical_non_additive_not_reclaimable",
            });
        }
    }
    Ok(Inventory { items, accounting: "Overlaps Docker virtual disk and filesystem parents. Do not sum items, shared layers, engine totals or filesystem totals. No physical reclaim estimate." })
}

#[cfg(test)]
mod tests;
