# PHASE 3: THE BACKEND API
## Week 4: The Web Interface for Machines

### Goal
A REST API and WebSocket server that serves the dashboard and receives webhooks from notification adapters. The API talks to the same SQLite database as the Rust core.

### What You Build

#### Component 1: The REST API (Axum)
A Rust web server using the Axum framework. It provides these endpoints:
- `GET /health`: Returns 200 OK if the core and database are healthy. Used by load balancers and monitoring.
- `GET /approvals`: List all approval requests with pagination, filtering (status, function name, date range), and sorting.
- `GET /approvals/:id`: Get a single approval request with full details.
- `POST /approvals/:id/approve`: Approve a pending request. Requires authentication.
- `POST /approvals/:id/deny`: Deny a pending request. Requires authentication.
- `GET /policies`: List all loaded policy files with their last modified time and validation status.
- `GET /policies/:name`: Get the content of a specific policy file.
- `POST /policies/:name/validate`: Validate a policy file content without saving it. Returns errors if invalid.
- `GET /audit-log`: Query the audit log with filters and pagination.
- `GET /metrics`: Prometheus-compatible metrics endpoint (request counts, latency histograms, active approvals).

All endpoints return JSON. All endpoints use consistent error formatting (RFC 7807 Problem Details).

#### Component 2: Authentication Middleware
A middleware layer that validates JWT tokens or Clerk session tokens. It supports two modes:
- Self-hosted mode: Validates JWT tokens from a configurable issuer (your own auth server, or no auth for single-user mode)
- Hosted mode: Validates Clerk session tokens using Clerk's JWKS endpoint

The middleware attaches the user ID and role to the request context. Role-based access control (RBAC) restricts certain endpoints (approving/denying requests requires the "approver" role, editing policies requires the "admin" role).

#### Component 3: WebSocket Server
A WebSocket endpoint (`/ws`) that pushes real-time updates to connected dashboard clients. When an approval request is created, approved, or denied, the Rust core emits an event. The API server receives the event via a Tokio broadcast channel and pushes it to all connected WebSocket clients.

The WebSocket server uses Axum's built-in WebSocket support. It handles connection limits (max 100 concurrent connections per API instance), heartbeat pings (every 30 seconds), and graceful disconnection.

#### Component 4: Webhook Receiver
An endpoint (`POST /webhooks/:adapter`) that receives incoming webhooks from notification adapters. When a human clicks "Approve" in a Slack message, Slack sends a webhook to this endpoint. The endpoint:
1. Validates the webhook signature (HMAC verification for Slack, JWT for custom adapters)
2. Parses the payload to extract the approval ID and decision
3. Calls the Rust core's signal handler to resolve the approval
4. Returns 200 OK to the adapter

The webhook receiver is idempotent. If Slack retries the webhook (because the first request timed out), the receiver recognizes the duplicate and returns 200 OK without double-processing.

#### Component 5: Rate Limiting and Security
- Rate limiting: 100 requests per minute per IP for public endpoints, 1000 per minute for authenticated endpoints. Uses a token bucket algorithm with Redis or in-memory storage.
- CORS: Configurable allowed origins. Defaults to localhost only in development.
- Security headers: HSTS, X-Content-Type-Options, X-Frame-Options, CSP.
- Input validation: All path parameters, query parameters, and body JSON are validated against schemas. Rejects malformed requests with 400 Bad Request.

### What You Do NOT Build in Phase 3
- No dashboard UI (this is the API only)
- No policy file editing through API (read-only for now)
- No team management or user provisioning
- No billing or subscription management
- No advanced analytics or reporting

### Success Criteria
- API responds to 95% of requests in under 50 milliseconds (excluding database queries)
- WebSocket pushes updates to clients within 100 milliseconds of the event
- Webhook receiver handles 10 concurrent Slack webhooks without dropping any
- API server starts in under 2 seconds
- All endpoints have OpenAPI documentation (auto-generated from Axum handlers)

### Human Tasks (Pratik)
- Write the Axum router and handlers
- Write the authentication middleware
- Write the WebSocket server
- Write the webhook receiver with HMAC verification
- Write OpenAPI documentation
- Write API integration tests (using reqwest and a test database)

### AI Assistance
- Use AI to generate Axum handler boilerplate and middleware chains
- Use AI to generate OpenAPI documentation from handler signatures
- Use AI to suggest security test cases (SQL injection, path traversal, replay attacks)
- Do NOT use AI to design the authentication flow. Clerk integration has specific security requirements. Follow Clerk's documentation exactly.
