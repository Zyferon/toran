# PHASE 4: THE WEB DASHBOARD
## Week 5-6: The Human Interface

### Goal
A web application where team members can:
1. See a live queue of pending approval requests
2. Click Approve or Deny with one click
3. Browse the audit log with search and filters
4. View policy files (read-only for now)
5. Configure notification adapters (Slack, email)

### What You Build

#### Component 1: The Approval Queue Page
A real-time table showing all pending approval requests. Columns:
- Function name (e.g., "send_email")
- Arguments summary (e.g., "to: boss@company.com, subject: Wire Transfer")
- Agent ID (which agent made the request)
- Risk score (color-coded: green for low, yellow for medium, red for high)
- Time waiting (how long since the request was made)
- Action buttons (Approve, Deny)

The table updates in real-time via WebSocket. When a new request arrives, it appears at the top with a subtle animation. When a request is resolved, it fades out and moves to the "Resolved" tab.

The table supports filtering by function name, status, date range, and agent ID. It supports sorting by any column. It uses TanStack Table for virtualization (handles 10,000 rows without lag).

#### Component 2: The Approval Detail View
A modal or page that shows the full details of a single approval request:
- Full function arguments (pretty-printed JSON)
- The policy rule that triggered the approval requirement
- The agent's previous actions in this session (context for the reviewer)
- Approve/Deny buttons with a confirmation dialog
- A "Deny and Block Agent" button (emergency stop)
- A comment field (optional note from the reviewer)

#### Component 3: The Audit Log Page
A searchable, filterable table of all past decisions (approved, denied, blocked, timed out). Columns:
- Timestamp
- Function name
- Decision
- Reviewer name (who approved/denied)
- Policy rule matched
- Agent ID
- Session ID

The audit log supports CSV export (for compliance officers) and JSON export (for developers). It uses server-side pagination (100 rows per page) to handle millions of records.

#### Component 4: The Policy Browser
A read-only view of all policy files. Shows:
- File name and last modified time
- Validation status (green checkmark if valid, red X if invalid)
- A preview of the file content with YAML syntax highlighting (using Prism or Shiki)
- A link to the raw file (for download)

Policy editing is disabled in Phase 4. Users edit policy files in their code editor and deploy via Git. The dashboard only displays them.

#### Component 5: The Notification Settings Page
A form for configuring notification adapters:
- Slack: Webhook URL input, channel selection, test button
- Email: SMTP server settings, from address, test button
- Webhook: Custom URL input, headers, test button
- Discord: Webhook URL input, test button

Each adapter has a "Test" button that sends a test notification to verify the configuration. The page shows the status of each adapter (connected, error, disabled).

#### Component 6: The Dashboard Layout
A sidebar navigation with links to:
- Queue (live approval requests)
- Audit Log (past decisions)
- Policies (policy file browser)
- Settings (notification adapters, team members)

A header bar with:
- Toran logo
- Real-time connection status (WebSocket connected/disconnected)
- User avatar and logout button
- A bell icon with a badge showing the number of pending approvals

### What You Do NOT Build in Phase 4
- No policy editing in the browser (YAML editing is error-prone; use Git for now)
- No user management or role assignment (single admin role for now)
- No analytics charts or graphs
- No mobile-responsive design (desktop-only for MVP)
- No dark mode (light mode only for MVP)

### Success Criteria
- Dashboard loads in under 2 seconds on a modern laptop
- Approval queue updates within 1 second of a real-time event
- Approving a request takes 2 clicks and under 3 seconds total
- Audit log search returns results in under 500 milliseconds
- Dashboard works in Chrome, Firefox, and Safari (latest versions)

### Human Tasks (Dipendra)
- Set up the Next.js project with shadcn/ui
- Build the approval queue page with TanStack Table and WebSocket integration
- Build the approval detail modal
- Build the audit log page with search and filters
- Build the policy browser with syntax highlighting
- Build the notification settings form
- Write Playwright end-to-end tests for the approval flow

### AI Assistance
- Use AI to generate React component boilerplate (forms, tables, modals)
- Use AI to generate Tailwind CSS styling suggestions
- Use AI to generate TypeScript type definitions from the API OpenAPI spec
- Use AI to suggest UX improvements (what should the reviewer see first?)
- Do NOT use AI to design the approval workflow. This is the core user experience. Design it yourself based on how you would want to review an agent's request.
