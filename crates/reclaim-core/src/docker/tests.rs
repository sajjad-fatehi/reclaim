use super::*;
use serde_json::json;

fn fixture() -> serde_json::Value {
    json!({
        "Images": [{"Id": format!("sha256:{}", "a".repeat(64)), "Size": 1000, "SharedSize": 700, "Containers": 1}],
        "Containers": [{"Id": "b".repeat(64), "State": "running", "SizeRw": 40}],
        "Volumes": [{"Name": "synthetic-persistent", "UsageData": {"Size": -1, "RefCount": 1}}],
        "BuildCache": [{"ID": "syntheticcache123", "Size": 300, "InUse": true, "Shared": true}]
    })
}

#[test]
fn four_kinds_preserve_uncertainty_and_do_not_double_count() {
    let inventory = parse_inventory(&serde_json::to_vec(&fixture()).unwrap()).unwrap();
    assert_eq!(inventory.items.len(), 4);
    assert_eq!(inventory.items[0].shared_bytes.as_deref(), Some("700"));
    assert_eq!(inventory.items[0].references, Some(1));
    assert_eq!(inventory.items[1].active, Some(true));
    assert!(inventory.items[1].persistent);
    assert!(!inventory.items[1].artifact.regenerable);
    assert!(inventory.items[2].logical_bytes.is_none());
    assert!(inventory.items[2].persistent);
    assert_eq!(inventory.items[3].shared, Some(true));
    for item in inventory.items {
        assert_eq!(
            item.artifact.cleanup_support,
            CleanupSupport::InspectionOnly
        );
        assert_eq!(item.artifact.exclusive_allocated_bytes, "0");
        assert_eq!(item.artifact.exclusive_logical_bytes, "0");
        assert!(item.artifact.project_id.is_none());
    }
}

#[test]
fn null_lists_are_empty_but_missing_or_invalid_fields_fail_closed() {
    assert!(
        parse_inventory(br#"{"Images":null,"Containers":null,"Volumes":[],"BuildCache":[]}"#)
            .unwrap()
            .items
            .is_empty()
    );
    for bytes in [b"{}".as_slice(), b"[]", b"not json", b"{\"Images\":[]}"] {
        assert_eq!(parse_inventory(bytes).unwrap_err(), Failure::Malformed);
    }
    for (field, value) in [
        ("Size", json!(-2)),
        ("Id", json!("bad")),
        ("SharedSize", json!(1001)),
    ] {
        let mut usage = fixture();
        usage["Images"][0][field] = value;
        assert_eq!(
            parse_inventory(&serde_json::to_vec(&usage).unwrap()).unwrap_err(),
            Failure::Malformed
        );
    }
    let mut usage = fixture();
    usage["Containers"][0]["State"] = json!("unknown");
    assert_eq!(
        parse_inventory(&serde_json::to_vec(&usage).unwrap()).unwrap_err(),
        Failure::Malformed
    );
    assert_eq!(
        parse_inventory(&vec![b' '; 8 * 1024 * 1024 + 1]).unwrap_err(),
        Failure::Limited
    );
}

#[test]
fn validates_engine_version_and_api_before_usage() {
    for (platform, version, api, expected) in [
        (
            "Docker Engine - Community",
            "24.0.9",
            "1.43",
            Ok("1.43".into()),
        ),
        ("Docker Desktop 4.40", "29.0.0", "1.52", Ok("1.52".into())),
        (
            "Podman Engine",
            "24.0.0",
            "1.43",
            Err(Failure::UnsupportedEngine),
        ),
        (
            "Docker Engine",
            "23.0.0",
            "1.42",
            Err(Failure::UnsupportedVersion),
        ),
        (
            "Docker Engine",
            "30.0.0",
            "1.53",
            Err(Failure::UnsupportedVersion),
        ),
    ] {
        assert_eq!(
            validate_version(
                &serde_json::to_vec(
                    &json!({"Platform":{"Name": platform},"Version":version,"ApiVersion":api})
                )
                .unwrap()
            ),
            expected
        );
    }
}

#[test]
fn unused_stopped_resources_still_preserve_persistent_data_and_duplicates_fail() {
    let mut usage = fixture();
    usage["Images"][0]["Containers"] = json!(0);
    usage["Containers"][0]["State"] = json!("exited");
    usage["Containers"][0]["SizeRw"] = serde_json::Value::Null;
    usage["Volumes"][0]["UsageData"] = json!({"Size": 120, "RefCount": 0});
    usage["BuildCache"][0]["InUse"] = json!(false);
    usage["BuildCache"][0]["Shared"] = json!(false);
    let result = parse_inventory(&serde_json::to_vec(&usage).unwrap()).unwrap();
    assert_eq!(result.items[0].references, Some(0));
    assert_eq!(result.items[1].active, Some(false));
    assert!(result.items[1].logical_bytes.is_none());
    assert_eq!(result.items[2].references, Some(0));
    assert_eq!(result.items[2].logical_bytes.as_deref(), Some("120"));
    assert_eq!(result.items[3].active, Some(false));
    assert!(result.items[2].persistent);
    let image = usage["Images"][0].clone();
    usage["Images"].as_array_mut().unwrap().push(image.clone());
    assert_eq!(
        parse_inventory(&serde_json::to_vec(&usage).unwrap()).unwrap_err(),
        Failure::Malformed
    );
    usage["Images"] = json!(vec![image; 4097]);
    assert_eq!(
        parse_inventory(&serde_json::to_vec(&usage).unwrap()).unwrap_err(),
        Failure::Limited
    );
}
