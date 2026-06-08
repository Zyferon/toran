# PHASE 6: LAUNCH & DOCUMENTATION
## Week 8: Ship It

### Goal
A public GitHub repository with a working product, clear documentation, and a launch plan.

### What You Build

#### Component 1: The README
A README.md that follows the "GitHub README Formula" for developer tools:
- One-sentence description at the top ("Runtime human approval gates for AI agents")
- A 2-minute demo video (GIF or Loom link) showing the decorator, the Slack notification, and the approval
- Installation instructions (pip install, binary download, Docker)
- A 5-minute quickstart (write a policy, decorate a function, trigger an approval)
- Feature list (framework-agnostic, sub-millisecond, self-hosted, open source)
- Comparison table (Toran vs Langfuse vs Braintrust vs LangGraph HITL)
- Link to full documentation
- License badge (MIT)
- GitHub stars badge
- CI status badge

#### Component 2: The Documentation Site
A Mintlify documentation site with:
- Introduction (what is Toran, why it exists, who it is for)
- Quickstart (5-minute setup)
- Concepts (policies, evaluation, blocking, notifications, audit trails)
- Guides (LangChain, CrewAI, Pydantic AI, AutoGen, custom scripts, Slack, email, self-hosting)
- SDK Reference (auto-generated from Python docstrings)
- CLI Reference (auto-generated from Rust CLI help)
- Contributing Guide (development setup, coding standards, PR process)
- FAQ (20 common questions)

#### Component 3: The Launch Assets
- A 2-minute demo video (screen recording with voiceover, no fancy editing)
- A 1-minute "What is Toran?" video (animated or narrated slide deck)
- 3 blog posts:
  - "Why I Built a Human-in-the-Loop Gate That Works With Any AI Agent Framework"
  - "The EU AI Act Requires Human Oversight. Here is a 5-Minute Setup."
  - "LangGraph Has Human-in-the-Loop. But Only If You Use LangGraph."
- A Twitter thread (10 tweets) announcing the launch
- A Hacker News "Show HN" post draft
- A Product Hunt launch page (tagline, description, screenshots, maker comments)

#### Component 4: The Community
- A Discord server with channels: #general, #help, #showcase, #feedback, #compliance-discussion
- A GitHub Discussions page enabled on the repo
- A "Good First Issue" label on 5 beginner-friendly issues
- A CONTRIBUTING.md with clear guidelines

#### Component 5: The Pricing Page (Simple)
A single page on the documentation site:
- Free: Open source, self-hosted, unlimited personal use
- Team ($49/month): Dashboard, team management, Slack integration, email support
- Enterprise ($199/month): SSO, audit exports, custom integrations, priority support, SLA

No complex pricing calculator. No usage-based billing. Flat monthly. Simple.

### What You Do NOT Build in Phase 6
- No paid signup flow (Stripe integration comes after first 5 manual customers)
- No marketing website (the documentation site is the marketing site)
- No blog platform (use Dev.to, Medium, or your own static site)
- No paid ads (organic only for now)
- No conference talks or podcasts (too early)

### Success Criteria
- GitHub repo has 100+ stars within 30 days of launch
- 3 real users who say "I would pay for this" (not "this is cool")
- 1 blog post with 1,000+ views
- 50 Discord members
- 10 "Good First Issue" contributions from external developers

### Human Tasks (Both)
- Pratik: Write the README, CLI reference, and technical blog posts
- Dipendra: Write the documentation site, record the demo video, design the pricing page
- Both: Launch on Hacker News, Twitter, Reddit, Product Hunt. Respond to every comment and question personally.

### AI Assistance
- Use AI to generate the documentation site structure and navigation
- Use AI to suggest blog post titles and outlines
- Use AI to generate the comparison table (research competitors' features)
- Use AI to suggest Hacker News title variations
- Do NOT use AI to write the README opening sentence. That is your hook. Write it yourself. It must sound like you, not like a marketing bot.
