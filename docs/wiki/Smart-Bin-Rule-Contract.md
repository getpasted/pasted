# Smart Bin Rule Contract

Smart Bins are computed collections. Manual membership is preserved separately and is unioned with rule matches.

## Version 1

```json
{
  "version": 1,
  "conditions": [
    { "type": "content_type", "operator": "is", "value": "link" },
    { "type": "source", "operator": "contains", "value": "Safari" }
  ],
  "match": "any"
}
```

`match` is `any` or `all`. Each condition uses `is` for a case-insensitive exact match or `contains` for a case-insensitive partial match. Rules contain 1–32 conditions, and each trimmed value is limited to 2,048 characters.

Current condition types are:

| Type | Value contract |
| --- | --- |
| `clip_type` | Structural `text`, `image`, or `file` value. |
| `content_type` | Stable Content Type ID assigned to the clip's current original or extracted searchable text. |
| `file_format` | Format ID produced by current byte-verified File Format inspection. |
| `source` | Captured source application name. |

Stable IDs are stored rather than localized labels. GUI pickers present localized built-in Content Type names and user-authored custom names without changing the saved ID.

The corresponding Functionality setting controls each condition type. Disabling Clip Types, Content Types, File Formats, or Sources makes conditions on that axis inactive without deleting the rule. Re-enabling the feature restores matching; rescanning can backfill derived Content Types and File Formats.

Legacy `origin_kind`, `contains`, `file_extension`, and `file_path` conditions remain readable for existing libraries and portable imports, but new authoring surfaces use the four collection axes above.

## Public boundaries

The GUI, Tauri commands, CLI, and History and Organization import validate rules through the shared Rust contract. Invalid versions, targets, operators, match modes, empty values, unknown fields, and excessive bounds are rejected instead of being saved as silently empty Smart Bins.

The CLI accepts this object through `--smart-rule-json`; `--json` output retains the canonical JSON string in `smart_rule`. Full Backup preserves the underlying SQLite record exactly.
