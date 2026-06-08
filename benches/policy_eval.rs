use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use toran::policy::compiler::compile_policy;
use toran::policy::evaluator::{Evaluator, Request};
use toran::policy::schema::{Action, PolicyFile, RuleFile, ToolPattern};

fn build_policy(n: usize) -> PolicyFile {
    let mut rules: Vec<RuleFile> = Vec::with_capacity(n);
    for i in 0..n {
        let tool = if i % 3 == 0 {
            ToolPattern::Exact(format!("func_{i}"))
        } else if i % 3 == 1 {
            ToolPattern::Glob {
                glob: "send_*".into(),
            }
        } else {
            ToolPattern::Regex {
                regex: "db_.*_write".into(),
            }
        };
        rules.push(RuleFile {
            name: format!("rule_{i}"),
            description: String::new(),
            tool,
            conditions: vec![],
            action: if i % 5 == 0 {
                Action::Block
            } else {
                Action::Allow
            },
            timeout_secs: None,
            risk_score: None,
        });
    }
    PolicyFile {
        name: "bench".into(),
        description: "bench".into(),
        priority: 0,
        default_action: Some("BLOCK".into()),
        rules,
    }
}

fn bench_eval(c: &mut Criterion) {
    let file = build_policy(1000);
    let compiled = compile_policy(&file);
    let ev = Evaluator::new();
    let mut args = HashMap::new();
    args.insert("to".into(), serde_json::json!("alice@x.io"));
    let req = Request {
        function_name: "send_email".into(),
        args,
        context: HashMap::new(),
    };
    c.bench_function("eval_1000rules", |b| {
        b.iter(|| {
            let d = ev.evaluate(&req, std::slice::from_ref(&compiled), Action::Block);
            std::hint::black_box(d);
        })
    });
}

criterion_group!(benches, bench_eval);
criterion_main!(benches);
