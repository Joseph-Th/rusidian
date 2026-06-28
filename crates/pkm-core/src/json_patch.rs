//! JSON Patch (RFC 6902) support for diffs.
//!
//! Provides utilities for creating and applying RFC 6902-compliant JSON patches.
//! Used to store space-efficient diffs instead of full before/after snapshots.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single operation in a JSON Patch (RFC 6902).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOp {
    pub op: String,      // "add", "remove", "replace", etc.
    pub path: String,    // JSON pointer to the location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>, // for "add" and "replace"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>, // for "copy" and "move"
}

/// Create a JSON Patch by comparing two JSON values.
/// Returns a minimal patch that transforms `before` into `after`.
/// Uses json-patch crate for RFC 6902 compliance.
pub fn create_patch(before: &Value, after: &Value) -> Vec<PatchOp> {
    if before == after {
        return Vec::new();
    }

    // Use json_patch::diff for minimal RFC 6902-compliant patches
    let diff = json_patch::diff(before, after);

    // Convert from json_patch operations to our PatchOp format
    let mut ops = Vec::new();
    for op in diff.0 {
        let patch_op = match op {
            json_patch::PatchOperation::Add(add_op) => PatchOp {
                op: "add".to_string(),
                path: add_op.path,
                value: Some(add_op.value),
                from: None,
            },
            json_patch::PatchOperation::Remove(remove_op) => PatchOp {
                op: "remove".to_string(),
                path: remove_op.path,
                value: None,
                from: None,
            },
            json_patch::PatchOperation::Replace(replace_op) => PatchOp {
                op: "replace".to_string(),
                path: replace_op.path,
                value: Some(replace_op.value),
                from: None,
            },
            json_patch::PatchOperation::Move(move_op) => PatchOp {
                op: "move".to_string(),
                path: move_op.path,
                value: None,
                from: Some(move_op.from),
            },
            json_patch::PatchOperation::Copy(copy_op) => PatchOp {
                op: "copy".to_string(),
                path: copy_op.path,
                value: None,
                from: Some(copy_op.from),
            },
            json_patch::PatchOperation::Test(test_op) => PatchOp {
                op: "test".to_string(),
                path: test_op.path,
                value: Some(test_op.value),
                from: None,
            },
        };
        ops.push(patch_op);
    }

    ops
}

/// Apply a JSON Patch to a value, returning the patched result.
pub fn apply_patch(value: &Value, patch: &[PatchOp]) -> Result<Value, String> {
    let mut result = value.clone();

    // Convert our PatchOp array to json_patch::Patch object via JSON value conversion
    let patch_value = serde_json::to_value(patch)
        .map_err(|e| format!("Failed to serialize patch ops: {}", e))?;
    let patch_ops: json_patch::Patch = serde_json::from_value(patch_value)
        .map_err(|e| format!("Failed to parse patch ops: {}", e))?;

    json_patch::patch(&mut result, &patch_ops).map_err(|e| e.to_string())?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn patch_replace_field() {
        let before = json!({"name": "Alice", "age": 30});
        let after = json!({"name": "Bob", "age": 30});

        let patch = vec![PatchOp {
            op: "replace".to_string(),
            path: "/name".to_string(),
            value: Some(json!("Bob")),
            from: None,
        }];

        let result = apply_patch(&before, &patch).unwrap();
        assert_eq!(result, after);
    }

    #[test]
    fn patch_add_field() {
        let before = json!({"name": "Alice"});
        let mut after = before.clone();
        after["city"] = json!("NYC");

        let patch = vec![PatchOp {
            op: "add".to_string(),
            path: "/city".to_string(),
            value: Some(json!("NYC")),
            from: None,
        }];

        let result = apply_patch(&before, &patch).unwrap();
        assert_eq!(result, after);
    }
}
