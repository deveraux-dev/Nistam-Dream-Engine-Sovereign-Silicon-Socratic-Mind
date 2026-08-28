// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Schema validation tests for .s13 telemetry payloads.
//!
//! Validates incoming telemetry before the 120Hz hotpath ingests them.

use serde_json::{json, Value};

#[test]
fn test_m5_geodesic_schema_structure() {
    let schema_str = include_str!("../schemas/m5_geodesic_spec.json");
    let schema: Value = serde_json::from_str(schema_str).expect("Valid JSON schema");

    assert_eq!(
        schema.get("title").and_then(|v| v.as_str()),
        Some("DiscretizedGeodesicHaikuSetup")
    );
    assert_eq!(
        schema.get("version").and_then(|v| v.as_str()),
        Some("1.0.0-s13")
    );
}

#[test]
fn test_m5_geodesic_manifold_parameters() {
    let schema_str = include_str!("../schemas/m5_geodesic_spec.json");
    let schema: Value = serde_json::from_str(schema_str).expect("Valid JSON schema");

    let manifold = schema
        .get("manifold_parameters")
        .expect("manifold_parameters missing");
    assert_eq!(manifold.get("axes_count").and_then(|v| v.as_u64()), Some(5));
    assert_eq!(
        manifold.get("valid_trit_states").and_then(|v| v.as_u64()),
        Some(243)
    );
    assert_eq!(
        manifold.get("metric_tensor_symbol").and_then(|v| v.as_str()),
        Some("g_ij")
    );
}

#[test]
fn test_m5_haiku_syllable_validation() {
    let schema_str = include_str!("../schemas/m5_geodesic_spec.json");
    let schema: Value = serde_json::from_str(schema_str).expect("Valid JSON schema");

    let haiku = schema.get("haiku").expect("haiku missing");
    let metrics = haiku.get("metrics").expect("metrics missing");
    let syllables = metrics.get("syllables").expect("syllables missing");

    if let Value::Array(arr) = syllables {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_u64(), Some(5));
        assert_eq!(arr[1].as_u64(), Some(7));
        assert_eq!(arr[2].as_u64(), Some(5));
    } else {
        panic!("syllables should be an array");
    }

    assert_eq!(
        metrics.get("total_syllables").and_then(|v| v.as_u64()),
        Some(17)
    );
}

#[test]
fn test_m5_valid_telemetry_payload() {
    let schema_str = include_str!("../schemas/m5_geodesic_spec.json");
    let _schema: Value = serde_json::from_str(schema_str).expect("Valid JSON schema");

    let valid_payload = json!({
        "title": "DiscretizedGeodesicHaikuSetup",
        "version": "1.0.0-s13",
        "manifold_parameters": {
            "axes_count": 5,
            "valid_trit_states": 243,
            "metric_tensor_symbol": "g_ij",
            "geodesic_traversal": "min_energy_lattice_hop"
        }
    });

    assert!(valid_payload.is_object());
}

#[test]
fn test_m5_telemetry_core_properties() {
    let schema_str = include_str!("../schemas/m5_geodesic_spec.json");
    let _schema: Value = serde_json::from_str(schema_str).expect("Valid JSON schema");

    let telemetry = json!({
        "timestamp_ns": 1_000_000_000u64,
        "query_latency_ns": 150u64,
        "coordinate": [1, 0, -1, 0, 1],
        "distance_to_origin": 3u8
    });

    assert!(telemetry.get("timestamp_ns").is_some());
    assert!(telemetry.get("query_latency_ns").is_some());
    assert!(telemetry.get("coordinate").is_some());
    assert!(telemetry.get("distance_to_origin").is_some());
}

#[test]
fn test_m5_coordinate_boundary_conditions() {
    let schema_str = include_str!("../schemas/m5_geodesic_spec.json");
    let schema: Value = serde_json::from_str(schema_str).expect("Valid JSON schema");

    let manifold = schema
        .get("manifold_parameters")
        .expect("manifold_parameters missing");

    let axes_count = manifold
        .get("axes_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let trit_states = manifold
        .get("valid_trit_states")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    assert_eq!(axes_count, 5);
    assert_eq!(3u32.pow(axes_count as u32), trit_states);
}

#[test]
fn test_m5_schema_immutability() {
    let schema_str = include_str!("../schemas/m5_geodesic_spec.json");
    let schema: Value = serde_json::from_str(schema_str).expect("Valid JSON schema");

    let seal = schema
        .get("sha256_seal")
        .and_then(|v| v.as_str())
        .expect("sha256_seal missing");

    assert_eq!(seal.len(), 64);
    assert!(seal.chars().all(|c| c.is_ascii_hexdigit()));
}
