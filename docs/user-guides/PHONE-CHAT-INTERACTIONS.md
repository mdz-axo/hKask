---
title: "Phone & Chat Agent Interactions"
audience: "User"
last_updated: 2026-06-12
version: "1.1.0"
status: "Active"
domain: "User Guide"
mds_categories: [capability, composition]
---

# Phone & Chat Agent Interactions

Your replicant can reach you via **SMS, WhatsApp, and voice calls** — turning it from a terminal-bound chatbot into an assistant that works alongside you, even when you're away from your keyboard.

The core idea: **let the agent handle the syntax so you can focus on the semantics.** Your replicant collates, sorts, classifies, monitors, and delivers. You decide what it means and what to do next.

---

## How Naming Works

You give your replicant a **first name** only — like "Bob" or "Carol." The system automatically composes the full name as:

> `[your chosen name] r[your last name]`

So if you're Alice Smith and you name your replicant "Bob," its full identity is **Bob rSmith**. This tells anyone who receives a message or call exactly whose assistant is contacting them. You never choose the last name — the system handles it.

---

## Setup

1. **Complete onboarding** (`kask chat` or `kask onboard`). You'll provide your name, phone number, and email, then name your replicant.
2. **Get a Telnyx API key** from [telnyx.com](https://telnyx.com) and set it:
   ```bash
   export HKASK_TELNYX_API_KEY="your-key"
   ```
3. **Give your replicant a phone number** (coming soon — currently set via `HKASK_REPLICANT_PHONE`).
4. **Start the Telnyx server** for your replicant:
   ```bash
   kask pod assign <replicant> telnyx
   kask pod mode <replicant> server -r telnyx
   ```

Your replicant can now use `telnyx_notify_user` to reach you.

---

## What Your Replicant Can Do For You

### Close the Loop on Tasks

> *"Track my action items from today's meetings. At 5pm, text me what's still open and who's waiting on what."*

The replicant does the syntax: it extracts commitments from your conversation history, categorizes them by owner and deadline, checks which ones haven't been resolved, and delivers a structured summary. You do the semantics: you know which loose ends actually matter and what to escalate.

### Monitor What You'd Otherwise Forget

> *"Watch my portfolio. Call me if any holding moves more than 5% in a day."*

The replicant handles the syntax of monitoring: polling FMP endpoints, computing deltas, checking thresholds. You handle the semantics: when the call comes, you decide whether to buy, sell, or hold. The agent doesn't need to understand *why* a 5% drop matters — it just needs to know that it does.

### Synthesize Before You Decide

> *"Every morning at 7am, text me: my calendar for the day, top 3 priorities, and any overnight emails that need a response before 9am."*

The replicant does the syntax: it pulls from your calendar, scans your inbox, ranks by urgency, and composes a brief. You do the semantics: you read the brief and decide where to put your attention. The agent replaced 20 minutes of collating with a 30-second read.

### Escalate When It Matters

> *"Remind me about the 2pm board meeting at 1:30. If I don't confirm by 1:45, call me. If I still don't answer, text my assistant."*

The replicant handles the escalation syntax: SMS → no response → call → no answer → notify backup. You handle the semantics: you're the one who knows the meeting is critical and who the backup should be. The agent executes the chain; you define the thresholds.

### Research While You're Doing Something Else

> *"I'm in a meeting. Text me the top 3 competitors to [company] and their latest funding rounds."*

You fire off a request. The replicant researches, ranks, and delivers. You glance at your phone between agenda items. The agent did the searching, sorting, and classifying. You do the meaning-making when you're ready.

### Coordinate Across Your Team of Replicants

> *"Have Bob rSmith (finance) text me the Q2 budget variance. Have Carol rSmith (research) WhatsApp me the competitor analysis for the board deck."*

Each replicant owns a domain. Each has its own number. Each handles the syntax of its specialty. You orchestrate across them — the human is the integration layer.

---

## The Four Patterns

These patterns appear across every production agent deployment in 2026. They're what turns a chatbot into an assistant.

### Pattern 1: Scheduled Push

```
You: "Send me X every [time] via [channel]"
Agent: collates → formats → delivers on schedule
You: read, decide, act
```

**The agent owns:** data gathering, formatting, timing, delivery.
**You own:** what the data means, what action to take.

Best for: daily briefings, portfolio summaries, calendar digests, news roundups.

### Pattern 2: Threshold Alert

```
You: "Alert me if [condition] via [channel]"
Agent: monitors → detects breach → notifies immediately
You: assess, decide, respond
```

**The agent owns:** continuous monitoring, condition evaluation, notification routing.
**You own:** threshold definition, significance judgment, response decision.

Best for: stock movements, price changes, deadline warnings, availability alerts.

### Pattern 3: On-Demand Query

```
You (via SMS/WhatsApp): "What's [question]?"
Agent: researches → synthesizes → replies same channel
You: read, follow up if needed
```

**The agent owns:** search, extraction, synthesis, formatting.
**You own:** question framing, relevance judgment, next action.

Best for: quick facts, comparisons, translations, calculations, recommendations.

### Pattern 4: Escalation Chain

```
Agent: SMS reminder
No response in N minutes → WhatsApp follow-up
Still no response → Phone call
Still no answer → Notify backup contact
```

**The agent owns:** the escalation sequence, timing, channel switching.
**You own:** defining what's escalation-worthy, who the backups are, when to stop.

Best for: critical meetings, time-sensitive decisions, safety checks, deadline enforcement.

---

## Why This Works

Research from production deployments in 2025-2026 shows a consistent pattern: when people switch from *conversational AI* (chatting with a bot) to *autonomous agents* (delegating to a system that plans, executes, and delivers), three things happen:

1. **Dissatisfaction drops by ~55%.** Users stop micromanaging steps and focus on verification and direction.
2. **Task complexity rises.** Users engage at higher cognitive levels — creating, evaluating, synthesizing — instead of just retrieving facts.
3. **The work frontier expands.** Tasks that were too tedious to do manually (monitoring 20 stocks, tracking 50 action items) become trivial to delegate.

The replicant doesn't replace your judgment. It replaces the **syntactic labor** — the collating, sorting, classifying, monitoring, reminding, and delivering — so your attention goes to the **semantic labor** that only you can do.

---

## Tips

- **One replicant, one domain.** A finance replicant, a research replicant, a personal assistant replicant. Each has its own number, its own expertise, its own memory. You orchestrate across them.
- **Set quiet hours.** Tell your replicant: "Don't contact me between 10pm and 7am unless it's urgent." The agent respects boundaries.
- **Use the right channel for the right signal.** SMS for one-shot facts. WhatsApp for ongoing threads. Calls for urgency. The replicant learns your preferences over time.
- **Be specific about what "done" looks like.** "Tell me when it's ready" is vague. "Text me a ranked list with sources" is executable. The upfront cost of clear goals pays off in zero marginal cost per delivery.

---

## Coming Soon

- **Phone number purchase during onboarding** — your replicant gets its own number automatically.
- **WhatsApp Business profile setup** — full WhatsApp identity for your replicant.
- **Scheduled message queue** — `telnyx_schedule_message` for precise delivery timing.
- **Conversation threads** — `telnyx_thread_history` to review past exchanges and maintain context across days.
