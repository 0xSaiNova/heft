//! Safety tier classification for bloat entries.
//!
//! Every entry gets a tier that determines picker behavior and --auto eligibility.
//! Tier 1 (Disposable) and Tier 2 (Rebuildable) can be auto-selected when stale.
//! Tier 3 (Caution) and Tier 4 (UserData) require manual selection.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyTier {
    Disposable,
    Rebuildable,
    Caution,
    UserData,
}

impl SafetyTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyTier::Disposable => "disposable",
            SafetyTier::Rebuildable => "rebuildable",
            SafetyTier::Caution => "caution",
            SafetyTier::UserData => "user_data",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "disposable" => Some(SafetyTier::Disposable),
            "rebuildable" => Some(SafetyTier::Rebuildable),
            "caution" => Some(SafetyTier::Caution),
            "user_data" => Some(SafetyTier::UserData),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SafetyTier::Disposable | SafetyTier::Rebuildable => "safe",
            SafetyTier::Caution => "caution",
            SafetyTier::UserData => "user data",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyInfo {
    pub tier: SafetyTier,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_dirty_files: Option<u32>,
}
