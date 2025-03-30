use serde_json::{Map, Value};
use std::collections::HashSet;

pub fn three_way_merge(base: &Value, a: &Value, b: &Value) -> (Value, bool) {
    three_way_merge_recursive(base, a, b, "")
}

fn three_way_merge_recursive(base: &Value, a: &Value, b: &Value, path: &str) -> (Value, bool) {
    match (base, a, b) {
        (Value::Object(base_map), Value::Object(a_map), Value::Object(b_map)) => {
            let mut merged = Map::new();
            let mut had_conflict = false;
            let keys: HashSet<String> = base_map
                .keys()
                .chain(a_map.keys())
                .chain(b_map.keys())
                .map(|k| k.to_string())
                .collect();

            for key in keys {
                let base_val = base_map.get(&key);
                let a_val = a_map.get(&key);
                let b_val = b_map.get(&key);

                let current_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}/{}", path, key)
                };

                let (merged_val, conflict) = merge_entry(base_val, a_val, b_val, &current_path);
                if conflict {
                    had_conflict = true;
                }

                if let Some(val) = merged_val {
                    merged.insert(key, val);
                }
            }
            (Value::Object(merged), had_conflict)
        }

        _ => {
            if a == b {
                (a.clone(), false)
            } else if a == base {
                (b.clone(), false)
            } else if b == base {
                (a.clone(), false)
            } else {
                log::error!("Conflict detected: path '{}' has different contents", path);
                (a.clone(), true)
            }
        }
    }
}

fn merge_entry(
    base: Option<&Value>,
    a: Option<&Value>,
    b: Option<&Value>,
    path: &str,
) -> (Option<Value>, bool) {
    match (base, a, b) {
        (Some(base_val), Some(a_val), Some(b_val)) => {
            if a_val == b_val {
                (Some(a_val.clone()), false)
            } else if a_val == base_val {
                (Some(b_val.clone()), false)
            } else if b_val == base_val {
                (Some(a_val.clone()), false)
            } else if a_val.is_object() && b_val.is_object() && base_val.is_object() {
                let (merged_val, conflict) =
                    three_way_merge_recursive(base_val, a_val, b_val, path);
                (Some(merged_val), conflict)
            } else {
                log::error!(
                    "Conflict: file '{}' modified in both branches with different contents",
                    path
                );
                (Some(a_val.clone()), true)
            }
        }

        (None, Some(a_val), Some(b_val)) => {
            if a_val == b_val {
                (Some(a_val.clone()), false)
            } else {
                log::error!(
                    "Conflict: file '{}' added in both branches with different contents",
                    path
                );
                (Some(a_val.clone()), true)
            }
        }

        (None, Some(a_val), None) => (Some(a_val.clone()), false),
        (None, None, Some(b_val)) => (Some(b_val.clone()), false),

        (Some(base_val), Some(a_val), None) => {
            if a_val == base_val {
                (None, false)
            } else {
                log::error!(
                    "Conflict: file '{}' modified in branch A but deleted in branch B",
                    path
                );
                (Some(a_val.clone()), true)
            }
        }

        (Some(base_val), None, Some(b_val)) => {
            if b_val == base_val {
                (None, false)
            } else {
                log::error!(
                    "Conflict: file '{}' modified in branch B but deleted in branch A",
                    path
                );
                (Some(b_val.clone()), true)
            }
        }

        (Some(_), None, None) => (None, false),

        (None, None, None) => panic!(
            "Unexpected case: file '{}' doesn't exist in any version",
            path
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_no_changes() {
        let base = json!({
            "file1.txt": "id1",
            "file2.txt": "id2"
        });
        let a = base.clone();
        let b = base.clone();

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, base);
        assert_eq!(had_conflicts, false);
    }

    #[test]
    fn test_changes_to_different_files() {
        let base = json!({
            "file1.txt": "id1",
            "file2.txt": "id2"
        });

        let a = json!({
            "file1.txt": "id1-modified",
            "file2.txt": "id2"
        });

        let b = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-modified"
        });

        let expected = json!({
            "file1.txt": "id1-modified",
            "file2.txt": "id2-modified"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, false);
    }

    #[test]
    fn test_same_changes_in_both_branches() {
        let base = json!({
            "file1.txt": "id1",
            "file2.txt": "id2"
        });

        let a = json!({
            "file1.txt": "id1-same-change",
            "file2.txt": "id2"
        });

        let b = json!({
            "file1.txt": "id1-same-change",
            "file2.txt": "id2"
        });

        let expected = json!({
            "file1.txt": "id1-same-change",
            "file2.txt": "id2"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, false);
    }

    #[test]
    fn test_conflict_in_same_file() {
        let base = json!({
            "file1.txt": "id1",
            "file2.txt": "id2"
        });

        let a = json!({
            "file1.txt": "id1-a-change",
            "file2.txt": "id2"
        });

        let b = json!({
            "file1.txt": "id1-b-change",
            "file2.txt": "id2"
        });

        // In conflict, branch A's value is used
        let expected = json!({
            "file1.txt": "id1-a-change",
            "file2.txt": "id2"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, true);
    }

    #[test]
    fn test_file_added_in_one_branch() {
        let base = json!({
            "file1.txt": "id1"
        });

        let a = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-new"
        });

        let b = base.clone();

        let expected = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-new"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, false);
    }

    #[test]
    fn test_file_added_in_both_branches_same_content() {
        let base = json!({
            "file1.txt": "id1"
        });

        let a = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-new"
        });

        let b = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-new"
        });

        let expected = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-new"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, false);
    }

    #[test]
    fn test_file_added_in_both_branches_different_content() {
        let base = json!({
            "file1.txt": "id1"
        });

        let a = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-new-a"
        });

        let b = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-new-b"
        });

        // In conflict, branch A's value is used
        let expected = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-new-a"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, true);
    }

    #[test]
    fn test_file_deleted_in_one_branch() {
        let base = json!({
            "file1.txt": "id1",
            "file2.txt": "id2"
        });

        let a = json!({
            "file1.txt": "id1"
        });

        let b = base.clone();

        let expected = json!({
            "file1.txt": "id1"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, false);
    }

    #[test]
    fn test_file_deleted_in_both_branches() {
        let base = json!({
            "file1.txt": "id1",
            "file2.txt": "id2"
        });

        let a = json!({
            "file1.txt": "id1"
        });

        let b = json!({
            "file1.txt": "id1"
        });

        let expected = json!({
            "file1.txt": "id1"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, false);
    }

    #[test]
    fn test_modified_in_one_branch_deleted_in_other() {
        let base = json!({
            "file1.txt": "id1",
            "file2.txt": "id2"
        });

        let a = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-modified"
        });

        let b = json!({
            "file1.txt": "id1"
        });

        // In conflict, branch A's change is kept
        let expected = json!({
            "file1.txt": "id1",
            "file2.txt": "id2-modified"
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, true);
    }

    #[test]
    fn test_nested_directory_structure() {
        let base = json!({
            "dir1": {
                "file1.txt": "id1",
                "file2.txt": "id2"
            },
            "dir2": {
                "file3.txt": "id3"
            }
        });

        let a = json!({
            "dir1": {
                "file1.txt": "id1-modified",
                "file2.txt": "id2"
            },
            "dir2": {
                "file3.txt": "id3"
            }
        });

        let b = json!({
            "dir1": {
                "file1.txt": "id1",
                "file2.txt": "id2"
            },
            "dir2": {
                "file3.txt": "id3-modified"
            }
        });

        let expected = json!({
            "dir1": {
                "file1.txt": "id1-modified",
                "file2.txt": "id2"
            },
            "dir2": {
                "file3.txt": "id3-modified"
            }
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, false);
    }

    #[test]
    fn test_nested_conflict() {
        let base = json!({
            "dir1": {
                "file1.txt": "id1",
                "file2.txt": "id2"
            }
        });

        let a = json!({
            "dir1": {
                "file1.txt": "id1-a-change",
                "file2.txt": "id2"
            }
        });

        let b = json!({
            "dir1": {
                "file1.txt": "id1-b-change",
                "file2.txt": "id2"
            }
        });

        // In conflict, branch A's value is used
        let expected = json!({
            "dir1": {
                "file1.txt": "id1-a-change",
                "file2.txt": "id2"
            }
        });

        let (merged, had_conflicts) = three_way_merge(&base, &a, &b);
        assert_eq!(merged, expected);
        assert_eq!(had_conflicts, true);
    }
}
