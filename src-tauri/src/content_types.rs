#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContentTypeDefinition {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub group: String,
    pub is_builtin: bool,
    pub is_archived: bool,
    #[serde(default)]
    pub defaults: Option<ContentTypeDefaults>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContentTypeDefaults {
    pub label: String,
    pub icon: String,
    pub group: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentTypeInput {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub group: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContentTypeGroupDefinition {
    pub id: String,
    pub label: String,
    pub sort_order: i64,
    pub is_builtin: bool,
    pub is_archived: bool,
    #[serde(default)]
    pub defaults: Option<ContentTypeGroupDefaults>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContentTypeGroupDefaults {
    pub label: String,
    pub sort_order: i64,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentTypeGroupInput {
    pub id: String,
    pub label: String,
    pub sort_order: i64,
}

#[derive(Clone, Copy)]
pub struct ContentTypeGroupPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub sort_order: i64,
}

pub const CONTENT_TYPE_GROUP_PRESETS: &[ContentTypeGroupPreset] = &[
    ContentTypeGroupPreset {
        id: "general",
        label: "General",
        sort_order: 10,
    },
    ContentTypeGroupPreset {
        id: "developer",
        label: "Developer",
        sort_order: 20,
    },
    ContentTypeGroupPreset {
        id: "personal_financial",
        label: "Personal and financial",
        sort_order: 30,
    },
    ContentTypeGroupPreset {
        id: "identifiers",
        label: "Identifiers",
        sort_order: 40,
    },
    ContentTypeGroupPreset {
        id: "custom",
        label: "Custom",
        sort_order: 50,
    },
];

#[derive(Clone, Copy)]
pub struct ContentTypePreset {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub group: &'static str,
}

pub const CONTENT_TYPE_PRESETS: &[ContentTypePreset] = &[
    ContentTypePreset {
        id: "text",
        label: "Plain Text",
        icon: "Type",
        group: "general",
    },
    ContentTypePreset {
        id: "prose",
        label: "Prose",
        icon: "ScrollText",
        group: "general",
    },
    ContentTypePreset {
        id: "link",
        label: "Web Link",
        icon: "Link",
        group: "general",
    },
    ContentTypePreset {
        id: "email",
        label: "Email Address",
        icon: "Mail",
        group: "general",
    },
    ContentTypePreset {
        id: "phone",
        label: "Phone Number",
        icon: "Phone",
        group: "general",
    },
    ContentTypePreset {
        id: "image",
        label: "Image",
        icon: "Image",
        group: "general",
    },
    ContentTypePreset {
        id: "file",
        label: "File",
        icon: "Files",
        group: "general",
    },
    ContentTypePreset {
        id: "file_path",
        label: "File Path",
        icon: "MapPin",
        group: "general",
    },
    ContentTypePreset {
        id: "color",
        label: "Color",
        icon: "Palette",
        group: "general",
    },
    ContentTypePreset {
        id: "code",
        label: "Code",
        icon: "Code",
        group: "developer",
    },
    ContentTypePreset {
        id: "shell_command",
        label: "Shell Command",
        icon: "TerminalSquare",
        group: "developer",
    },
    ContentTypePreset {
        id: "env_variable",
        label: "Environment Variable",
        icon: "Variable",
        group: "developer",
    },
    ContentTypePreset {
        id: "env_block",
        label: "Environment Block",
        icon: "FileCode2",
        group: "developer",
    },
    ContentTypePreset {
        id: "credential",
        label: "Credential",
        icon: "KeyRound",
        group: "personal_financial",
    },
    ContentTypePreset {
        id: "payment_card",
        label: "Payment Card",
        icon: "CreditCard",
        group: "personal_financial",
    },
    ContentTypePreset {
        id: "iban",
        label: "IBAN",
        icon: "Landmark",
        group: "personal_financial",
    },
    ContentTypePreset {
        id: "jwt",
        label: "JSON Web Token",
        icon: "ShieldKeyhole",
        group: "identifiers",
    },
    ContentTypePreset {
        id: "hash",
        label: "Hash",
        icon: "Hash",
        group: "identifiers",
    },
    ContentTypePreset {
        id: "ip_address",
        label: "IP Address",
        icon: "Network",
        group: "identifiers",
    },
    ContentTypePreset {
        id: "mac_address",
        label: "MAC Address",
        icon: "Router",
        group: "identifiers",
    },
    ContentTypePreset {
        id: "uuid",
        label: "UUID",
        icon: "Fingerprint",
        group: "identifiers",
    },
];

pub const CONTENT_TYPE_ICONS: &[&str] = &[
    "AlignLeft",
    "AtSign",
    "Binary",
    "BookOpen",
    "Box",
    "Braces",
    "Calendar",
    "CheckSquare",
    "CircleDollarSign",
    "Clipboard",
    "Clock",
    "Database",
    "Code",
    "CreditCard",
    "FileCode2",
    "FileText",
    "Files",
    "Fingerprint",
    "Hash",
    "Image",
    "KeyRound",
    "Landmark",
    "Link",
    "Mail",
    "MapPin",
    "Network",
    "Palette",
    "Phone",
    "Router",
    "ScrollText",
    "ShieldKeyhole",
    "TerminalSquare",
    "Type",
    "Variable",
    "FileJson",
    "FileSpreadsheet",
    "Folder",
    "Globe",
    "Heart",
    "List",
    "Lock",
    "MessageSquare",
    "Package",
    "Receipt",
    "Search",
    "Settings",
    "Star",
    "Tag",
    "User",
    "Wallet",
    "Wrench",
    "Zap",
];

pub fn content_type_defaults(id: &str) -> Option<ContentTypeDefaults> {
    CONTENT_TYPE_PRESETS
        .iter()
        .find(|preset| preset.id == id)
        .map(|preset| ContentTypeDefaults {
            label: preset.label.into(),
            icon: preset.icon.into(),
            group: preset.group.into(),
        })
}

pub fn content_type_group_defaults(id: &str) -> Option<ContentTypeGroupDefaults> {
    CONTENT_TYPE_GROUP_PRESETS
        .iter()
        .find(|preset| preset.id == id)
        .map(|preset| ContentTypeGroupDefaults {
            label: preset.label.into(),
            sort_order: preset.sort_order,
        })
}

pub fn fallback_label(id: &str) -> String {
    id.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn validate_content_type_input(input: &ContentTypeInput) -> Result<(), String> {
    if !valid_registry_id(&input.id) {
        return Err("Type IDs must use 1–80 lowercase letters, numbers, and underscores".into());
    }
    if input.label.trim().is_empty() || input.label.chars().count() > 80 {
        return Err("Type names must contain 1–80 characters".into());
    }
    if !CONTENT_TYPE_ICONS.contains(&input.icon.as_str()) {
        return Err("Unknown content type icon".into());
    }
    if !valid_registry_id(&input.group) {
        return Err(
            "Type Group IDs must use 1–80 lowercase letters, numbers, and underscores".into(),
        );
    }
    Ok(())
}

pub fn validate_content_type_group_input(input: &ContentTypeGroupInput) -> Result<(), String> {
    if !valid_registry_id(&input.id) {
        return Err("Group IDs must use 1–80 lowercase letters, numbers, and underscores".into());
    }
    if input.label.trim().is_empty() || input.label.chars().count() > 80 {
        return Err("Group names must contain 1–80 characters".into());
    }
    if !(-10_000..=10_000).contains(&input.sort_order) {
        return Err("Group sort order must be between -10000 and 10000".into());
    }
    Ok(())
}

fn valid_registry_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}
