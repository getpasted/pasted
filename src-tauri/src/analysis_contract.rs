use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationKind {
    ClipKind,
    OriginalText,
    #[serde(rename = "image")]
    ImageBytes,
    SearchableText,
    AnalyzableText,
    Classification,
}

impl RepresentationKind {
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::ClipKind => "clip_kind",
            Self::OriginalText => "original_text",
            Self::ImageBytes => "image",
            Self::SearchableText => "searchable_text",
            Self::AnalyzableText => "analyzable_text",
            Self::Classification => "classification",
        }
    }
}

impl fmt::Display for RepresentationKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.stable_name())
    }
}

impl FromStr for RepresentationKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "clip_kind" => Ok(Self::ClipKind),
            "original_text" => Ok(Self::OriginalText),
            "image" => Ok(Self::ImageBytes),
            "searchable_text" => Ok(Self::SearchableText),
            "analyzable_text" => Ok(Self::AnalyzableText),
            "classification" => Ok(Self::Classification),
            _ => Err(format!("Unknown analysis representation \"{value}\"")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepresentationContract {
    pub input: RepresentationKind,
    pub output: RepresentationKind,
}

impl RepresentationContract {
    pub fn parse(input: &str, output: &str) -> Result<Self, String> {
        Ok(Self {
            input: input.parse()?,
            output: output.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representation_names_round_trip_through_the_shared_contract() {
        for kind in [
            RepresentationKind::ClipKind,
            RepresentationKind::OriginalText,
            RepresentationKind::ImageBytes,
            RepresentationKind::SearchableText,
            RepresentationKind::AnalyzableText,
            RepresentationKind::Classification,
        ] {
            assert_eq!(kind.stable_name().parse::<RepresentationKind>(), Ok(kind));
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.stable_name())
            );
            assert_eq!(
                serde_json::from_str::<RepresentationKind>(&format!("\"{}\"", kind.stable_name()))
                    .unwrap(),
                kind
            );
        }
    }

    #[test]
    fn unknown_representation_names_fail_closed() {
        assert_eq!(
            RepresentationContract::parse("image", "mystery").unwrap_err(),
            "Unknown analysis representation \"mystery\""
        );
    }
}
