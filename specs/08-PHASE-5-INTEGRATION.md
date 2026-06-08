# PHASE 5: FRAMEWORK INTEGRATIONS & NOTIFICATIONS
## Week 7: Making It Useful in the Real World

### Goal
Integrate Toran with popular agent frameworks and notification channels. Make it work where developers actually build agents.

### What You Build

#### Framework Integration 1: LangChain
A small Python module (`toran.integrations.langchain`) that provides a `ToranTool` wrapper. This wrapper wraps any LangChain tool and adds the `@gate` decorator transparently. Users do not change their agent code. They only change their tool definitions:

```python
from toran.integrations.langchain import ToranTool
from langchain.tools import SendEmailTool

safe_email = ToranTool(SendEmailTool(), policy="email-guardian")
agent = initialize_agent(tools=[safe_email, ...], ...)
```

The wrapper intercepts the tool's `_run` method, calls the Toran evaluator, and handles approval before calling the original tool.

#### Framework Integration 2: CrewAI
A similar wrapper for CrewAI tools. CrewAI uses a different tool interface, so the wrapper adapts between CrewAI's `tool` decorator and Toran's `@gate` decorator.

#### Framework Integration 3: Pydantic AI
Pydantic AI uses dependency injection for tools. The integration provides a `toran_guard` dependency that wraps Pydantic AI's `RunContext` and adds policy evaluation before tool execution.

#### Framework Integration 4: AutoGen
AutoGen uses conversational agents with function calling. The integration provides a `ToranFunction` wrapper that wraps AutoGen's `register_function` and adds approval gates.

#### Framework Integration 5: Custom Scripts
A helper function `toran.wrap_function(func, policy)` that wraps any Python function without requiring a decorator. This is useful for users who cannot modify the function definition (e.g., third-party library functions).

#### Notification Adapter 1: Slack
A Slack app (not just a webhook) that:
- Sends interactive messages with Approve/Deny buttons
- Shows the full function arguments in a collapsible section
- Updates the message when the request is resolved (changes buttons to "Approved by @user at 10:30 AM")
- Supports slash commands (`/toran status` to see pending approvals)
- Supports DM notifications (send approval requests to individual users, not just channels)

The Slack adapter uses Slack's Block Kit API for rich interactive messages. It verifies all incoming webhooks with Slack's signing secret (HMAC-SHA256).

#### Notification Adapter 2: Email
An email adapter that sends HTML emails with:
- A summary of the request (function name, arguments, agent ID)
- Approve and Deny buttons (linked to the dashboard with pre-authenticated tokens)
- A link to the full dashboard for more context
- A footer with the company name and timestamp

The email adapter uses Resend API (free tier: 3,000 emails/month) or SMTP. It supports HTML and plain text fallback.

#### Notification Adapter 3: Generic Webhook
A webhook adapter that POSTs to a user-configured URL with the full request payload. The user can build their own notification system (e.g., PagerDuty, custom Slack bot, internal dashboard). The webhook includes an HMAC signature for verification.

#### Notification Adapter 4: Discord
A Discord webhook adapter that sends rich embeds to a channel with Approve/Deny buttons (using Discord's interaction API).

### What You Do NOT Build in Phase 5
- No Microsoft Teams adapter (lower priority)
- No SMS adapter (requires Twilio integration, complex regulations)
- No mobile push notifications (requires Apple/Google developer accounts)
- No framework integrations for non-Python languages (JavaScript, Go, Java)

### Success Criteria
- LangChain integration works with both `AgentExecutor` and `LangGraph` agents
- Slack adapter sends a message within 2 seconds of a REQUIRE_APPROVAL decision
- Email adapter delivers to inbox (not spam folder) for Gmail and Outlook
- Webhook adapter retries failed deliveries 3 times with exponential backoff
- All adapters handle 100 concurrent notifications without dropping any

### Human Tasks (Dipendra)
- Write all framework integration wrappers
- Write all notification adapters
- Write integration tests for each framework (spin up a real agent, trigger a tool, verify approval flow)
- Write Slack app configuration guide
- Test email deliverability (use Mail Tester to verify spam score)

### AI Assistance
- Use AI to generate the Slack Block Kit JSON for rich messages
- Use AI to generate the HTML email template
- Use AI to suggest retry logic and backoff strategies for webhooks
- Use AI to generate framework-specific wrapper code (LangChain, CrewAI, Pydantic AI interfaces)
- Do NOT use AI to design the webhook payload schema. This is your API contract. Design it carefully for backward compatibility.
