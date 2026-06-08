# SYSTEM ARCHITECTURE
## How Toran Works From 50,000 Feet

### The Five Layers

#### Layer 1: The Policy Layer (Human-Readable Rules)
Humans write rules in YAML or JSON. These rules live in your repository, next to your code, in version control. They are not hidden in a web dashboard behind a login wall. They are plain text. Diffable. Reviewable. Auditable.

A policy file defines:
- Which tools require approval
- Which tools are always allowed
- Which tools are always blocked
- Conditions based on function arguments (if the email contains "wire transfer," require approval)
- Escalation rules (if no one approves in 5 minutes, notify the manager)

#### Layer 2: The Evaluation Layer (Rust Core)
When an agent calls a tool, the Python SDK intercepts the call and sends the function name, arguments, and context to the Rust core. The Rust core does three things:
1. Loads the relevant policy file from disk or memory cache
2. Evaluates the rule against the incoming request
3. Returns a decision: ALLOW, BLOCK, or REQUIRE_APPROVAL

This evaluation happens in a dedicated thread pool inside a Rust process. The Python interpreter never blocks. The agent never waits. The Rust core is a separate binary that runs alongside your Python application, communicating via a local Unix socket or TCP socket.

#### Layer 3: The Blocking Layer (Async Wait)
If the decision is ALLOW, the Python function executes immediately. No delay. No overhead.

If the decision is BLOCK, the Python function returns a graceful error immediately. The agent can catch it and try an alternative.

If the decision is REQUIRE_APPROVAL, the Python function enters an async wait state. It pauses execution but does not consume a thread. It registers itself with a state manager (Redis or SQLite) and waits for a signal. The agent process stays alive. The memory stays allocated. The function simply sleeps until a human says yes or no.

#### Layer 4: The Notification Layer (Human Alert)
When approval is required, Toran sends a notification through whatever channel you configure:
- Slack message with Approve/Deny buttons
- Email with a secure link
- Webhook to your own internal system
- In-app notification if you embed the Toran dashboard
- SMS for critical actions

The notification contains:
- What the agent wants to do
- Why the policy triggered (which rule, which condition)
- The full context (arguments, agent ID, timestamp, risk score)
- A secure, one-time link to approve or deny

#### Layer 5: The Resolution Layer (Resume or Abort)
When a human clicks Approve, the signal travels back to the Rust core, which updates the state manager, which wakes up the sleeping Python function. The function executes its original body. The agent continues as if nothing happened.

When a human clicks Deny, the signal travels the same path, but the Python function receives a Denied exception. The agent catches it and handles the failure gracefully.

If no one responds within a timeout window (configurable per policy), the function receives a Timeout exception.

### Data Flow Diagram (Plain English)
1. Agent code calls a decorated function
2. Decorator intercepts the call before the function body runs
3. Decorator serializes the call metadata (name, args, kwargs, timestamp, agent ID)
4. Metadata travels via zero-copy shared memory or local socket to Rust core
5. Rust core evaluates policy in under 1 millisecond
6. Rust core returns decision to Python SDK
7. If ALLOW: decorator runs the original function body immediately
8. If BLOCK: decorator raises a Blocked exception immediately
9. If REQUIRE_APPROVAL: decorator registers an async future with the state manager, sends a notification, and suspends the function
10. Human receives notification and clicks Approve or Deny
11. Notification service sends webhook to Rust core
12. Rust core updates state manager
13. State manager wakes the async future
14. If approved: original function body runs. If denied: Denied exception raises.

### The Local-First Architecture
Toran is designed to run entirely on your laptop, your server, or your VPC without talking to our servers. The Rust core is a single binary. The state manager defaults to SQLite. The notification layer can use your existing Slack webhook. There is no "Toran Cloud" that you must use.

The hosted version (what we charge money for) adds:
- A managed dashboard with team management
- Audit log storage and search
- SAML/SSO authentication
- Priority support

But the core engine, the policy evaluation, the blocking mechanism — all of that is open source and runs on your hardware. If our company dies, your gatekeeper keeps working.
