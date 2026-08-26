# Activity and Insights

Activity and Insights describe the local library without sending analytics or clipboard data anywhere. They answer different questions: **Activity** is an audit history of important events, while **Insights** summarizes the current library and recent additions.

## Activity

Activity records privacy-safe events for capture, organization, deletion and recovery, Analysis, Transforms, settings, import, backup, and other meaningful operations. Entries contain bounded identifiers, counts, outcomes, and neutral summaries—not clipboard contents, transformation input, credentials, or sensitive file details.

The Activity view loads retained entries incrementally and supports text, category, outcome, and severity filtering. Severity is informational, warning, or error; outcome records whether the operation succeeded, failed, or produced another defined result. Clearing Activity permanently removes only audit entries and does not change clips or other library data.

Activity retention has independent count and age limits under **Settings → General**. JSON export uses a versioned OpenTelemetry-shaped event contract with timestamp, observed timestamp, event name, severity, body, attributes, and archive-level resource metadata. CSV is intended for reporting. Imports are bounded, transactional, deduplicated, and inert: importing an event never replays the action it describes. Current retention limits apply after import.

Use **Settings → Storage → Export or Import** for files, or `pasted activity list|export|import|clear` for scripts. See [CLI Reference](CLI-Reference#activity) for the complete command forms.

## Insights

Insights summarizes only the active library; clips in Trash are excluded. It separates structural **Clip Types**, byte-verified **File Formats**, semantic **Content Types**, and capture **Sources** so unlike concepts are not combined into one chart. A multi-file clip can contribute more than one File Format, while each clip is counted once per detected Content Type.

Recent-addition charts use the machine's local calendar days. The stored capture timestamps remain canonical UTC instants, so summaries continue to use the correct local day across time-zone and daylight-saving changes.

Insights also reports library totals and daily Activity when the corresponding data is available. Disabling **Insights** under **Settings → Functionality** hides this view without stopping capture, Analysis, Activity, or retention. Disabling a discovery axis hides that axis from Insights while preserving its stored results.

The scriptable equivalent is `pasted insights summary [--json]`.
