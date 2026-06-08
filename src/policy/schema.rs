//! Policy schema: the on-disk YAML shape, the compiled in-memory form,
//! and the public data types used by the evaluator.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One rule in a policy file.
///
/// A rule matches when:
///   1. The request's function name matches `tool` (exact string or
///      regex), AND
///   2. Every entry in `conditions` matches (AND of all).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleFile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub tool: ToolPattern,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub action: Action,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub risk_score: Option<u8>,
}

/// A pattern that matches a function (tool) name.
///
/// Accepts any of these YAML shapes:
///   tool: send_email                    # string -> Exact
///   tool: { exact: send_email }         # Exact
///   tool: { glob: "send_*" }            # Glob
///   tool: { regex: "^db_.*$" }          # Regex
///
/// We use a custom deserializer because the `untagged` attribute
/// alone cannot disambiguate a single-key map from another
/// single-key map (each variant is structurally a map).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPattern {
    Exact(String),
    Glob { glob: String },
    Regex { regex: String },
}

impl Serialize for ToolPattern {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Exact(v) => s.serialize_str(v),
            Self::Glob { glob } => s.serialize_newtype_struct("Glob", glob),
            Self::Regex { regex } => s.serialize_newtype_struct("Regex", regex),
        }
    }
}

impl<'de> Deserialize<'de> for ToolPattern {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = ToolPattern;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string or { exact | glob | regex } map")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(ToolPattern::Exact(v.to_string()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(ToolPattern::Exact(v))
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut m: M,
            ) -> Result<Self::Value, M::Error> {
                let key: Option<String> = m.next_key()?;
                let value: String = match key.as_deref() {
                    Some("exact") => m.next_value()?,
                    Some("glob") => {
                        return Ok(ToolPattern::Glob {
                            glob: m.next_value()?,
                        });
                    }
                    Some("regex") => {
                        return Ok(ToolPattern::Regex {
                            regex: m.next_value()?,
                        });
                    }
                    Some(other) => {
                        return Err(serde::de::Error::custom(format!(
                            "unknown tool pattern `{other}` (expected exact | glob | regex)"
                        )));
                    }
                    None => return Err(serde::de::Error::custom("empty tool pattern map")),
                };
                // Make sure no extra keys.
                if m.next_key::<String>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "tool pattern must have a single key",
                    ));
                }
                Ok(ToolPattern::Exact(value))
            }
        }
        d.deserialize_any(V)
    }
}

/// A single condition in a `when` clause.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Condition {
    pub key: String,
    pub op: Operator,
    pub value: ConditionValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    Ne,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    Gt,
    Lt,
    Gte,
    Lte,
    In,
    NotIn,
    Exists,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ConditionValue {
    String(String),
    Number(i64),
    Bool(bool),
    List(Vec<String>),
}

/// The decision a policy can produce.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    #[default]
    Allow,
    Block,
    RequireApproval,
}

impl Action {
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "ALLOW" => Some(Self::Allow),
            "BLOCK" => Some(Self::Block),
            "REQUIRE_APPROVAL" | "REQUIREAPPROVAL" | "REQUIRE" => Some(Self::RequireApproval),
            _ => None,
        }
    }
}

/// A policy file as it appears on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Higher priority wins when multiple policies match. Default 0.
    /// Use negative numbers for permissive fallbacks, positive for
    /// strict overrides.
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub default_action: Option<String>,
    pub rules: Vec<RuleFile>,
}

// ---------------------------------------------------------------------------
// Compiled form (hot-path)
// ---------------------------------------------------------------------------

/// A compiled rule with the regex already pre-parsed.
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub name: String,
    pub tool_matcher: ToolMatcher,
    pub conditions: Vec<CompiledCondition>,
    pub action: Action,
    pub timeout_secs: u64,
    pub risk_score: u8,
}

#[derive(Debug, Clone)]
pub enum ToolMatcher {
    Exact(String),
    Glob(glob::GlobMatcher),
    Regex(regex::Regex),
}

/// Avoid pulling in a glob crate: we only support `*` wildcards. This
/// is converted to a regex at compile time.
pub mod glob {
    #[derive(Debug, Clone)]
    pub struct GlobMatcher(pub regex::Regex);
    impl GlobMatcher {
        pub fn is_match(&self, s: &str) -> bool {
            self.0.is_match(s)
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledCondition {
    pub key: String,
    pub op: Operator,
    pub value: ConditionValue,
    /// Pre-compiled regex for `Operator::Regex`.
    pub compiled_regex: Option<regex::Regex>,
}

/// The compiled, in-memory representation of every loaded policy file.
#[derive(Debug, Clone, Default)]
pub struct CompiledPolicy {
    pub name: String,
    /// Higher wins when policies disagree. Mirrors the YAML field.
    pub priority: i32,
    pub default_action: Action,
    /// Bucket of rules by exact function name. Rules with glob/regex
    /// matchers are stored in `fallback` and tried if no exact match.
    pub by_name: HashMap<String, Vec<usize>>,
    pub fallback: Vec<usize>,
    pub rules: Vec<CompiledRule>,
}
