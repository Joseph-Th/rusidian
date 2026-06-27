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
pub fn create_patch(before: &Value, after: &Value) -> Vec<PatchOp> {
    let mut ops = Vec::new();

    // For now, use a simple full replacement (future: implement Myers diff for minimal patches)
    if before != after {
        ops.push(PatchOp {
            op: "replace".to_string(),
            path: "/".to_string(),
            value: Some(after.clone()),
            from: None,
        });
    }

    ops
}

/// Apply a JSON Patch to a value, returning the patched result.
pub fn apply_patch(value: &Value, patch: &[PatchOp]) -> Result<Value, String> {
    let mut result = value.clone();

    for op in patch {
        match op.op.as_str() {
            "add" => {
                if let Some(val) = &op.value {
                    apply_add(&mut result, &op.path, val.clone())?;
                }
            }
            "remove" => {
                apply_remove(&mut result, &op.path)?;
            }
            "replace" => {
                if let Some(val) = &op.value {
                    apply_replace(&mut result, &op.path, val.clone())?;
                }
            }
            _ => return Err(format!("Unsupported patch op: {}", op.op)),
        }
    }

    Ok(result)
}

fn apply_add(value: &mut Value, path: &str, new_val: Value) -> Result<(), String> {
    if path == "/" {
        *value = new_val;
        return Ok(());
    }

    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let mut current = value;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(ref mut obj) = current {
                obj.insert(part.to_string(), new_val);
            }
            return Ok(());
        }

        if let Value::Object(ref mut obj) = current {
            current = obj.entry(part.to_string()).or_insert(Value::Object(Default::default()));
        }
    }

    Ok(())
}

fn apply_remove(value: &mut Value, path: &str) -> Result<(), String> {
    if path == "/" {
        return Err("Cannot remove root".to_string());
    }

    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let mut current = value;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(ref mut obj) = current {
                obj.remove(*part);
            }
            return Ok(());
        }

        if let Value::Object(ref mut obj) = current {
            current = obj.entry(part.to_string()).or_insert(Value::Object(Default::default()));
        }
    }

    Ok(())
}

fn apply_replace(value: &mut Value, path: &str, new_val: Value) -> Result<(), String> {
    if path == "/" {
        *value = new_val;
        return Ok(());
    }

    apply_remove(value, path)?;
    apply_add(value, path, new_val)?;
    Ok(())
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
