//! Hot-path policy evaluator. Pure function: no side effects, no
//! allocation beyond the returned struct. Designed to complete in
//! under 1 millisecond for typical rulesets (10–1000 rules).
//!
//! Algorithm:
//!   1. Hash-lookup rules whose `tool_matcher` is `Exact(name)`.
//!      If any of those match all their conditions, return the
//!      first match (rules are ordered in the source file).
//!   2. Otherwise, scan the `fallback` bucket (glob/regex matchers)
//!      in order. First full match wins.
//!   3. If still no match, return the policy's `default_action`.

use super::schema::{
    Action, CompiledCondition, CompiledPolicy, ConditionValue, Operator, ToolMatcher,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A request from the Python SDK. The arguments are a flat key/value
/// map. Nested values are allowed via dotted keys (`address.city`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub function_name: String,
    #[serde(default)]
    pub args: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,
}

impl Request {
    /// Look up a value in args first, then context, with dotted-path
    /// support (e.g. "address.city").
    pub fn lookup(&self, key: &str) -> Option<&serde_json::Value> {
        if let Some(v) = self.args.get(key) {
            return Some(v);
        }
        if let Some(v) = self.context.get(key) {
            return Some(v);
        }
        let mut parts = key.split('.');
        let first = parts.next()?;
        let mut current = self.args.get(first).or_else(|| self.context.get(first))?;
        for seg in parts {
            current = current.get(seg)?;
        }
        Some(current)
    }

    /// Build a Request from raw JSON strings (used when reconstructing
    /// a Request from a stored approval record).
    pub fn from_json_strings(
        function_name: &str,
        arguments_json: &str,
        context_json: &str,
    ) -> Self {
        let args: HashMap<String, serde_json::Value> =
            serde_json::from_str(arguments_json).unwrap_or_default();
        let context: HashMap<String, serde_json::Value> =
            serde_json::from_str(context_json).unwrap_or_default();
        Self {
            function_name: function_name.to_string(),
            args,
            context,
        }
    }
}

/// The decision returned by the evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub action: Action,
    pub rule_name: String,
    pub risk_score: u8,
    pub timeout_secs: u64,
    pub elapsed_ns: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub evaluations: u64,
    pub total_ns: u128,
}

impl Stats {
    pub fn avg(&self) -> Duration {
        if self.evaluations == 0 {
            Duration::ZERO
        } else {
            Duration::from_nanos((self.total_ns / self.evaluations as u128) as u64)
        }
    }
}

pub struct Evaluator {
    pub stats: parking_lot::Mutex<Stats>,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator {
    pub const fn new() -> Self {
        Self {
            stats: parking_lot::Mutex::new(Stats {
                evaluations: 0,
                total_ns: 0,
            }),
        }
    }

    pub fn evaluate(
        &self,
        request: &Request,
        policies: &[CompiledPolicy],
        default_action: Action,
    ) -> Decision {
        let start = Instant::now();
        let mut decision = Decision {
            action: default_action,
            rule_name: "<default>".into(),
            risk_score: match default_action {
                Action::Allow => 10,
                Action::Block => 0,
                Action::RequireApproval => 80,
            },
            timeout_secs: 300,
            elapsed_ns: 0,
        };
        'outer: for policy in policies {
            // 1. Exact-name bucket
            if let Some(bucket) = policy.by_name.get(&request.function_name) {
                for &idx in bucket {
                    let rule = &policy.rules[idx];
                    if conditions_match(&rule.conditions, request) {
                        decision.action = rule.action;
                        decision.rule_name = format!("{}::{}", policy.name, rule.name);
                        decision.risk_score = rule.risk_score;
                        decision.timeout_secs = rule.timeout_secs;
                        break 'outer;
                    }
                }
            }
            // 2. Fallback (glob/regex)
            for &idx in &policy.fallback {
                let rule = &policy.rules[idx];
                if tool_matches(&rule.tool_matcher, &request.function_name)
                    && conditions_match(&rule.conditions, request)
                {
                    decision.action = rule.action;
                    decision.rule_name = format!("{}::{}", policy.name, rule.name);
                    decision.risk_score = rule.risk_score;
                    decision.timeout_secs = rule.timeout_secs;
                    break 'outer;
                }
            }
        }
        let elapsed = start.elapsed();
        decision.elapsed_ns = elapsed.as_nanos() as u64;
        let mut s = self.stats.lock();
        s.evaluations += 1;
        s.total_ns += elapsed.as_nanos();
        decision
    }
}

fn tool_matches(m: &ToolMatcher, name: &str) -> bool {
    match m {
        ToolMatcher::Exact(s) => s == name,
        ToolMatcher::Glob(g) => g.is_match(name),
        ToolMatcher::Regex(r) => r.is_match(name),
    }
}

pub fn conditions_match(conds: &[CompiledCondition], req: &Request) -> bool {
    // Short-circuit: an empty condition list always matches.
    if conds.is_empty() {
        return true;
    }
    conds.iter().all(|c| condition_matches(c, req))
}

fn condition_matches(c: &CompiledCondition, req: &Request) -> bool {
    let actual = req.lookup(&c.key);
    match c.op {
        Operator::Exists => actual.is_some(),
        Operator::Eq => actual.map(|v| json_eq(v, &c.value)).unwrap_or(false),
        Operator::Ne => !actual.map(|v| json_eq(v, &c.value)).unwrap_or(false),
        Operator::Contains => match (actual, &c.value) {
            (Some(serde_json::Value::String(s)), ConditionValue::String(needle)) => {
                s.contains(needle.as_str())
            }
            _ => false,
        },
        Operator::StartsWith => match (actual, &c.value) {
            (Some(serde_json::Value::String(s)), ConditionValue::String(p)) => {
                s.starts_with(p.as_str())
            }
            _ => false,
        },
        Operator::EndsWith => match (actual, &c.value) {
            (Some(serde_json::Value::String(s)), ConditionValue::String(p)) => {
                s.ends_with(p.as_str())
            }
            _ => false,
        },
        Operator::Regex => match (actual, &c.value) {
            (Some(serde_json::Value::String(s)), _) => c
                .compiled_regex
                .as_ref()
                .map(|r| r.is_match(s))
                .unwrap_or(false),
            _ => false,
        },
        Operator::Gt | Operator::Lt | Operator::Gte | Operator::Lte => {
            numeric_cmp(actual, &c.value, &c.op)
        }
        Operator::In => match (actual, &c.value) {
            (Some(v), ConditionValue::List(list)) => list.iter().any(|s| json_eq_str(v, s)),
            _ => false,
        },
        Operator::NotIn => match (actual, &c.value) {
            (Some(v), ConditionValue::List(list)) => !list.iter().any(|s| json_eq_str(v, s)),
            _ => true,
        },
    }
}

fn numeric_cmp(actual: Option<&serde_json::Value>, target: &ConditionValue, op: &Operator) -> bool {
    let a = match actual {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    };
    let b = match target {
        ConditionValue::Number(n) => Some(*n as f64),
        ConditionValue::String(s) => s.parse::<f64>().ok(),
        _ => None,
    };
    match (a, b) {
        (Some(a), Some(b)) => match op {
            Operator::Gt => a > b,
            Operator::Lt => a < b,
            Operator::Gte => a >= b,
            Operator::Lte => a <= b,
            _ => false,
        },
        _ => false,
    }
}

fn json_eq(a: &serde_json::Value, b: &ConditionValue) -> bool {
    match (a, b) {
        (serde_json::Value::String(s), ConditionValue::String(t)) => s == t,
        (serde_json::Value::Bool(x), ConditionValue::Bool(y)) => x == y,
        (serde_json::Value::Number(n), ConditionValue::Number(m)) => n.as_i64() == Some(*m),
        (serde_json::Value::String(s), ConditionValue::Number(n)) => {
            s.parse::<i64>().ok().map(|p| p == *n).unwrap_or(false)
        }
        _ => false,
    }
}

fn json_eq_str(a: &serde_json::Value, b: &str) -> bool {
    match a {
        serde_json::Value::String(s) => s == b,
        serde_json::Value::Number(n) => n.to_string() == b,
        serde_json::Value::Bool(x) => *x == (b == "true"),
        _ => false,
    }
}
