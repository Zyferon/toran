# SECURITY MODEL
## How Toran Protects Itself and You

### Threat Model
We assume the following threats and design against them:

#### Threat 1: Malicious Agent Bypasses the Gate
An attacker compromises the agent and tries to call a tool directly without going through the Toran decorator.

Mitigation:
- The decorator is a wrapper, not a proxy. The original function is only accessible through the decorator. If the attacker imports the original function directly, they bypass Toran. But if the developer only exposes the decorated function to the agent, the agent cannot bypass it.
- Defense in depth: The developer should also use network-level controls (firewall rules, API keys) on the tool itself. Toran is the gate, not the castle wall.

#### Threat 2: Policy File Injection
An attacker modifies the policy file to allow malicious actions.

Mitigation:
- Policy files are loaded from a read-only directory (chmod 444).
- Policy file changes are validated against JSON Schema before compilation. Invalid files are rejected.
- Policy file changes are logged to the audit trail with the user who made the change.
- In production, policy files should be deployed via Git (GitOps), not edited on the server.

#### Threat 3: Approval Spoofing
An attacker sends a fake approval signal to the Rust core, tricking it into executing a blocked function.

Mitigation:
- All approval signals include a cryptographically random token (256-bit, generated with `ring` crate's CSPRNG).
- The token is only sent to the notification channel (Slack, email) and the dashboard. The attacker cannot guess it.
- The Rust core verifies the token before resolving the approval. Invalid tokens are logged and rejected.
- Tokens expire after the timeout window (default 5 minutes, configurable).

#### Threat 4: Denial of Service
An attacker floods the Rust core with requests, causing it to drop legitimate approvals or run out of memory.

Mitigation:
- Connection limits: The socket server accepts a maximum number of concurrent connections (default 10,000). Additional connections are rejected immediately.
- Rate limiting: Per-IP and per-agent-ID rate limits prevent a single source from overwhelming the system.
- Resource quotas: Each suspended function has a memory cap. If an agent tries to suspend 1 million functions, the core rejects new suspensions after the quota.
- Timeouts: All async waits have a maximum timeout. After timeout, the function is automatically denied. No function can wait forever.

#### Threat 5: Eavesdropping on Socket Communication
An attacker on the same machine reads the Unix socket traffic between Python and Rust.

Mitigation:
- Unix sockets are protected by filesystem permissions (chmod 660, owned by the application user).
- For multi-tenant environments, use TCP sockets with TLS (mutual TLS authentication between Python and Rust).
- FlatBuffers serialization is not encryption, but it is not human-readable. An attacker needs to reverse-engineer the schema.

#### Threat 6: SQL Injection in Policy Conditions
An attacker writes a policy condition that evaluates to arbitrary SQL or code.

Mitigation:
- Policy conditions are not evaluated as code. They are evaluated as a pre-defined set of operations (equality, comparison, regex, set membership). There is no `eval()` function. No code execution.
- The policy compiler validates all conditions against the schema. Unknown operations are rejected at compile time, not evaluation time.
- The sandbox module uses a whitelist approach: only known-safe operations are allowed.

#### Threat 7: Tampered Audit Logs
An attacker modifies the audit log to hide a malicious action.

Mitigation:
- Audit logs are append-only. The Rust core opens the database in WAL mode and never updates existing records. Records are only inserted.
- For high-security deployments, enable tamper-evident logging: each log entry includes a hash of the previous entry's hash, forming a chain. Modifying one entry breaks the chain.
- For enterprise customers, audit logs can be streamed to an external immutable store (AWS S3 with Object Lock, WORM storage).

### Security Checklist for Production
- [ ] Run the Rust core as a non-root user
- [ ] Set filesystem permissions on the policy directory (read-only for core user)
- [ ] Enable TLS for all TCP communication
- [ ] Rotate approval tokens daily
- [ ] Enable audit log streaming to external storage
- [ ] Set resource quotas (max concurrent approvals, max memory)
- [ ] Enable Prometheus alerts for unusual patterns (spike in blocked requests, spike in approval timeouts)
- [ ] Run `cargo audit` weekly to check for vulnerable dependencies
- [ ] Run `bandit` and `safety` on the Python SDK weekly
- [ ] Conduct a penetration test before the first enterprise customer
