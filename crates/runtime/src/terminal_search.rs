use serde::{Deserialize, Serialize};
use terminal_emulator::StableRowIndex;

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SearchResult {
    pub start_y: StableRowIndex,
    pub start_x: usize,
    pub end_y: StableRowIndex,
    pub end_x: usize,
    pub match_id: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Pattern {
    CaseSensitiveString(String),
    CaseInSensitiveString(String),
    Regex(String),
}

impl Default for Pattern {
    fn default() -> Self {
        Self::CaseSensitiveString(String::new())
    }
}

impl std::ops::Deref for Pattern {
    type Target = String;

    fn deref(&self) -> &String {
        match self {
            Self::CaseSensitiveString(s) | Self::CaseInSensitiveString(s) | Self::Regex(s) => s,
        }
    }
}

impl std::ops::DerefMut for Pattern {
    fn deref_mut(&mut self) -> &mut String {
        match self {
            Self::CaseSensitiveString(s) | Self::CaseInSensitiveString(s) | Self::Regex(s) => s,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum PatternType {
    CaseSensitiveString,
    CaseInSensitiveString,
    Regex,
}

impl From<&Pattern> for PatternType {
    fn from(value: &Pattern) -> Self {
        match value {
            Pattern::CaseSensitiveString(_) => Self::CaseSensitiveString,
            Pattern::CaseInSensitiveString(_) => Self::CaseInSensitiveString,
            Pattern::Regex(_) => Self::Regex,
        }
    }
}
