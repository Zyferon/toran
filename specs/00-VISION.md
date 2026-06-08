# TORAN — VISION & PHILOSOPHY
## The Only Document That Matters

### What Toran Is
Toran is a runtime gatekeeper for AI agents. It does not log what happened. It does not observe after the fact. It stands at the threshold between an agent's intention and the real world, and it decides — in real time, in sub-millisecond latency — whether that intention is allowed to execute.

### What Toran Is Not
- Not a tracing tool. We do not compete with Langfuse, Braintrust, or Arize.
- Not a model evaluator. We do not judge whether an LLM output is "good."
- Not a cloud service you must rent from us. We are infrastructure you own.

### The Core Philosophy: Sovereignty
Every other AI governance tool wants you inside their platform. Their dashboard. Their cloud. Their pricing tier. They want to be the middleman between you and your own agent.

Toran says: **your agent's decisions belong to you. The policy that governs them belongs to you. The hardware that enforces them belongs to you.**

We are not a SaaS company pretending to be infrastructure. We are infrastructure that happens to have a hosted option.

### The Three Promises
1. **Zero Framework Lock-in**: Drop one decorator on any Python function. Any framework. Any loop. Any script. No rewrite required.
2. **Zero Latency Penalty**: Policy evaluation happens in under one millisecond. The agent does not notice we are there.
3. **Zero Vendor Lock-in**: The core engine is Rust. The policies are YAML. The state is SQLite or Redis or Postgres — your choice. If you stop paying us, everything keeps running.

### Why This Exists
LangGraph has human-in-the-loop, but only if you build your entire agent inside LangGraph. OpenAI has guardrails, but only if you use their stack. Every tool assumes you will rearrange your architecture to fit their worldview.

Toran assumes nothing. It assumes you have a Python function that sends an email, and you want a human to approve it before it sends. That is all. No graph. No state machine. No vendor.

### The Name
Toran (तोरण) is the decorated gateway in Nepali architecture. It hangs over the threshold. It does not own the house. It does not own the street. It simply marks the moment where the outside world meets the inside world — and it makes that meeting auspicious, deliberate, and safe.

That is what we do. We are the threshold.
