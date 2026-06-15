---
title: "ADR-038 — Media MCP Server Architecture"
audience: [architects, developers]
last_updated: 2026-06-14
version: "0.27.0"
status: "Draft"
domain: "Technology"
mds_categories: [composition, domain]
---

# ADR-038 — Media MCP Server Architecture

**Status:** Draft  
**Date:** 2026-06-14  
**References:** Design plan archived 2026-06-15. Handoffs in git history.

---

## Context

hKask generates media (image, video, audio, voice, collage, 3D) through MCP tools. The `hkask-mcp-media` crate serves 28 tools from a single MCP server. fal.ai is the primary generation backend.

Agent sessions developed the architecture across multiple handoffs (archived 2026-06-15, recoverable via git history). This ADR documents those decisions.

## Decision

### Single MCP Server, Multiple Tool Categories

A single MCP server (`hkask-mcp-media`) serves all media tools across six categories:
- **Image:** generate, edit, upscale, background-remove
- **Video:** generate, edit
- **Audio:** generate, transcribe
- **Voice:** clone, synthesize, convert
- **Collage:** compose, layout
- **3D:** generate, texture

### fal.ai as Primary Backend

fal.ai generates all media types. Local fallbacks sit in the P3 backlog — not implemented.

### Tool Naming Convention

Tools follow the pattern: `media/{category}/{action}` (e.g., `media/image/generate`, `media/voice/clone`).

### 12 Tests, Compiles Clean

The server has 12 tests covering tool registration, parameter validation, and output format verification. All tests pass; `cargo check` and `cargo clippy` are clean.

## Consequences

- **Positive:** One binary, one MCP connection — deployment is a single step.
- **Positive:** `media/{category}/{action}` naming makes tool discovery predictable.
- **Negative:** All generation needs network access and a fal.ai API key. No local fallback exists.
- **Negative:** 28 tools create a large surface. Future consolidation should reduce this.
- **Negative:** Architectural knowledge lived only in code and handoffs until this ADR. This document closes that gap.

## Procedural Rhetoric

- **PS-01 (Shared Goal):** Media generation for agent communication and content creation.
- **PS-02 (Bounded Lexicon):** 6 tool categories, fal.ai backend, `media/{category}/{action}` naming.
- **PS-03 (Mode of Play):** Single-server, multi-category; fal.ai as primary backend.
- **PS-12 (Invitational Voice):** New media categories integrate via additional tool registrations.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
