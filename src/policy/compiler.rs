//! Policy compiler: turn a parsed [`PolicyFile`] into a hot-path
//! [`CompiledPolicy`] with pre-parsed regexes and bucketed rules.

use super::schema::{
    Action, CompiledCondition, CompiledPolicy, CompiledRule, Condition, ConditionValue, Operator,
    PolicyFile, RuleFile, ToolMatcher, ToolPattern,
};
use anyhow::{Result, anyhow};
use regex::Regex;

pub fn compile_policy(file: &PolicyFile) -> CompiledPolicy {
    let mut by_name: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    let mut fallback: Vec<usize> = Vec::new();
    let mut rules: Vec<CompiledRule> = Vec::with_capacity(file.rules.len());

    // We need to bucket rules by tool, but inside each bucket we want
    // more-specific rules (those with conditions) before less-specific
    // ones (empty conditions act as fallbacks). To keep rule IDs
    // stable we first collect (original_index, rule_file) pairs, sort
    // each bucket by `has_conditions desc`, then assign sequential
    // IDs in bucket order.
    let mut bucketed: Vec<(ToolMatcher, &RuleFile)> = Vec::new();
    for r in &file.rules {
        let matcher = match &r.tool {
            ToolPattern::Exact(s) => ToolMatcher::Exact(s.clone()),
            ToolPattern::Glob { glob } => {
                let pattern = format!("^{}$", glob_to_regex(glob));
                match Regex::new(&pattern) {
                    Ok(re) => ToolMatcher::Glob(super::schema::glob::GlobMatcher(re)),
                    Err(e) => {
                        tracing::error!(rule = %r.name, error = %e, "skipping invalid glob");
                        continue;
                    }
                }
            }
            ToolPattern::Regex { regex } => match Regex::new(regex) {
                Ok(re) => ToolMatcher::Regex(re),
                Err(e) => {
                    tracing::error!(rule = %r.name, error = %e, "skipping invalid regex");
                    continue;
                }
            },
        };
        bucketed.push((matcher, r));
    }

    // Stable sort: rules with conditions come first within each matcher.
    bucketed.sort_by(|a, b| {
        let a_spec = !a.1.conditions.is_empty();
        let b_spec = !b.1.conditions.is_empty();
        b_spec.cmp(&a_spec)
    });

    for (matcher, r) in &bucketed {
        let compiled = match compile_rule(r) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(rule = %r.name, error = %e, "skipping invalid rule");
                continue;
            }
        };
        let idx = rules.len();
        match matcher {
            ToolMatcher::Exact(name) => by_name.entry(name.clone()).or_default().push(idx),
            ToolMatcher::Glob(_) | ToolMatcher::Regex(_) => fallback.push(idx),
        }
        rules.push(compiled);
    }

    CompiledPolicy {
        name: file.name.clone(),
        priority: file.priority,
        default_action: Action::Allow,
        by_name,
        fallback,
        rules,
    }
}

fn compile_rule(r: &RuleFile) -> Result<CompiledRule> {
    let tool_matcher = match &r.tool {
        ToolPattern::Exact(s) => ToolMatcher::Exact(s.clone()),
        ToolPattern::Glob { glob } => {
            // Translate "send_*" -> "^send_.*$"
            let pattern = format!("^{}$", glob_to_regex(glob));
            let re = Regex::new(&pattern).map_err(|e| anyhow!("bad glob {}: {e}", glob))?;
            ToolMatcher::Glob(super::schema::glob::GlobMatcher(re))
        }
        ToolPattern::Regex { regex } => {
            let re = Regex::new(regex).map_err(|e| anyhow!("bad regex {}: {e}", regex))?;
            ToolMatcher::Regex(re)
        }
    };

    let conditions = r
        .conditions
        .iter()
        .map(compile_condition)
        .collect::<Result<Vec<_>>>()?;

    Ok(CompiledRule {
        name: r.name.clone(),
        tool_matcher,
        conditions,
        action: r.action,
        timeout_secs: r.timeout_secs.unwrap_or(300),
        risk_score: r.risk_score.unwrap_or(match r.action {
            Action::Allow => 10,
            Action::Block => 0,
            Action::RequireApproval => 80,
        }),
    })
}

fn compile_condition(c: &Condition) -> Result<CompiledCondition> {
    let compiled_regex = if matches!(c.op, Operator::Regex) {
        if let ConditionValue::String(s) = &c.value {
            Some(Regex::new(s).map_err(|e| anyhow!("bad regex in condition: {e}"))?)
        } else {
            return Err(anyhow!("regex operator requires string value"));
        }
    } else {
        None
    };
    Ok(CompiledCondition {
        key: c.key.clone(),
        op: c.op.clone(),
        value: c.value.clone(),
        compiled_regex,
    })
}

fn glob_to_regex(g: &str) -> String {
    let mut out = String::with_capacity(g.len() + 4);
    for ch in g.chars() {
        match ch {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            '.' | '(' | ')' | '[' | ']' | '{' | '}' | '+' | '|' | '^' | '$' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}
