---
title: "hkask-condenser — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

# hkask-condenser — API Reference

**Purpose:** Context condensation engine. Compresses conversation context for LLM token budget management using multiple algorithms with health signal monitoring.

## Public Modules

| Module | Purpose |
|--------|---------|
| `engine` | `CondenserEngine` — the central compression orchestrator (7 tools, 51 tests) |
| `algorithms` | Compression algorithms: word-rank, RTK-style, FlashRank, LLM summarization |
| `types` | Core types: `CompressedOutput`, health signals, compression profiles |
| `inference` | LLM-based summarization via centralized inference router |
| `saliency` | Passage salience scoring and ranking |
| `ontology_graph` | Ontology-aware compression graph |

## Key Types

| Type | Description |
|------|-------------|
| `CondenserEngine` | Central compression engine with 7 tools |
| `CompressedOutput` | Result of a compression operation — compressed text + health signals |

## 7 Tools

| Tool | Function |
|------|----------|
| `condenser_ping` | Liveness check + profile info |
| `condenser_compress` | Context-aware compression with health signals |
| `condenser_set_profile` | Profile switching (heavy/normal/soft/light) |
| `condenser_stats` | Cumulative compression statistics |
| `condenser_classify` | Tool → category classification for context routing |
| `condenser_persist` | Persist compressed context to episodic memory |
| `condenser_thread_summary` | LLM summarization via centralized inference router |

## Health Signals

`CompressedOutput::health_signals` returns diagnostic signals for CNS monitoring:

| Signal | Trigger | Meaning |
|--------|---------|---------|
| `negative_compression` | `compressed_bytes > original_bytes` | RTK-style bounds violation |
| `low_signal` | >50% of lines score 0.0 | Word-rank found no usable signal |
| `budget_shortfall` | `filled < budget` | FlashRank couldn't find enough signal |
| `low_compression_ratio` | Overall ratio < 2:1 after 10+ compressions | Systemic SLA violation |

## Key Functions

| Function | Signature |
|----------|-----------|
| `approx_token_count` | Estimates token count for a text string |
| `build_summarization_prompt` | Constructs an LLM summarization prompt from context |
| `build_summary_output` | Parses LLM summary output into structured form |
| `format_conversation_text` | Formats conversation transcript for compression input |
