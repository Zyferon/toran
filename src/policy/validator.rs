//! Policy validator: structural checks beyond what serde gives us.

use super::schema::{Operator, PolicyFile};
use anyhow::{Result, anyhow};

pub fn validate(file: &PolicyFile) -> Result<()> {
    if file.name.trim().is_empty() {
        return Err(anyhow!("policy file is missing a `name`"));
    }
    if file.rules.is_empty() {
        return Err(anyhow!("policy '{}' has zero rules", file.name));
    }
    for r in &file.rules {
        if r.name.trim().is_empty() {
            return Err(anyhow!("rule in '{}' has empty name", file.name));
        }
        for c in &r.conditions {
            match (&c.op, &c.value) {
                (Operator::In | Operator::NotIn, super::schema::ConditionValue::List(_)) => {}
                (Operator::In | Operator::NotIn, _) => {
                    return Err(anyhow!(
                        "rule '{}' condition `{}` uses in/not_in with a non-list value",
                        r.name,
                        c.key
                    ));
                }
                (
                    Operator::Gt | Operator::Lt | Operator::Gte | Operator::Lte,
                    super::schema::ConditionValue::Number(_),
                ) => {}
                (Operator::Gt | Operator::Lt | Operator::Gte | Operator::Lte, _) => {
                    return Err(anyhow!(
                        "rule '{}' condition `{}` uses numeric op with non-number value",
                        r.name,
                        c.key
                    ));
                }
                (_, super::schema::ConditionValue::String(_)) => {}
                (_, super::schema::ConditionValue::Bool(_)) => {}
                (_, super::schema::ConditionValue::Number(_)) => {
                    return Err(anyhow!(
                        "rule '{}' condition `{}` numeric value with non-numeric op",
                        r.name,
                        c.key
                    ));
                }
                (_, super::schema::ConditionValue::List(_)) => {
                    return Err(anyhow!(
                        "rule '{}' condition `{}` list value with scalar op",
                        r.name,
                        c.key
                    ));
                }
            }
        }
    }
    Ok(())
}
