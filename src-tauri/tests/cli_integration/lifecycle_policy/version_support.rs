use super::super::support::*;
pub(super) fn flagged_version_id(versions: &[Value], field: &str, value: bool) -> String {
    versions
        .iter()
        .find(|version| {
            version[field] == value
                && version[if field == "is_original" {
                    "is_current"
                } else {
                    "is_original"
                }] == false
        })
        .expect("matching version")["id"]
        .as_i64()
        .unwrap()
        .to_string()
}
