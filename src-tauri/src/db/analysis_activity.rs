use super::*;

pub(super) fn analysis_toggle_activity(
    kind: &str,
    name: &str,
    enabled: bool,
) -> Option<(&'static str, String)> {
    let event_type = match (kind, enabled) {
        ("extractor", true) => "content_extractor_enabled",
        ("extractor", false) => "content_extractor_disabled",
        ("classifier", true) => "content_classifier_enabled",
        ("classifier", false) => "content_classifier_disabled",
        _ => return None,
    };
    let label = if kind == "extractor" {
        "Extractor"
    } else {
        "Classifier"
    };
    Some((
        event_type,
        format!(
            "{} {label} \"{name}\"",
            if enabled { "Enabled" } else { "Disabled" }
        ),
    ))
}

impl DbState {
    #[allow(clippy::type_complexity)]
    pub(super) fn log_analysis_participant_toggle(
        &self,
        kind: &str,
        stable_ref: &str,
        name: &str,
        enabled: bool,
    ) {
        let Some((event_type, description)) = analysis_toggle_activity(kind, name, enabled) else {
            return;
        };
        let _ = self.log_activity_with_attributes(
            event_type,
            &description,
            &serde_json::json!({
                "analysis.participant.kind": kind,
                "analysis.participant.ref": stable_ref,
                "analysis.participant.enabled": enabled,
            }),
        );
    }

    pub(super) fn log_analysis_participant_update(
        &self,
        kind: &str,
        stable_ref: &str,
        name: &str,
        previous_enabled: bool,
        enabled: bool,
    ) {
        if previous_enabled != enabled {
            self.log_analysis_participant_toggle(kind, stable_ref, name, enabled);
            return;
        }
        let (event_type, label) = if kind == "extractor" {
            ("content_extractor_updated", "Extractor")
        } else {
            ("content_classifier_updated", "Classifier")
        };
        let _ = self.log_activity(event_type, &format!("Updated {label} \"{name}\""));
    }
}
