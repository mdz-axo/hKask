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
**References:** `docs/plans/mcp-media-server-design.md`, handoffs: `media-voice-talk-2026-06-13.md`, `media-server-continuation-2026-06-14.md`

---

## Context

hKask requires media generation capabilities (image, video, audio, voice, collage, 3D) exposed as MCP tools. The `hkask-mcp-media` crate implements a 28-tool MCP server with fal.ai as the primary generation backend.

The architecture was developed across multiple agent sessions and specified in `mcp-media-server-design.md`. This ADR retroactively documents the architectural decisions.

## Decision

### Single MCP Server, Multiple Tool Categories

All media tools are served from a single MCP server (`hkask-mcp-media`) with tool categories:
- **Image:** generate, edit, upscale, background-remove
- **Video:** generate, edit
- **Audio:** generate, transcribe
- **Voice:** clone, synthesize, convert
- **Collage:** compose, layout
- **3D:** generate, texture

### fal.ai as Primary Backend

fal.ai is the primary generation backend for all media types. Local fallbacks are not implemented (P3 backlog).

### Tool Naming Convention

Tools follow the pattern: `media/{category}/{action}` (e.g., `media/image/generate`, `media/voice/clone`).

### 12 Tests, Compiles Clean

The server has 12 tests covering tool registration, parameter validation, and output format verification. All tests pass; `cargo check` and `cargo clippy` are clean.

## Consequences

- **Positive:** Single server simplifies deployment — one binary, one MCP connection.
- **Positive:** Consistent tool naming makes discovery predictable.
- **Negative:** No local fallback — all generation requires network + fal.ai API key.
- **Negative:** 28 tools is a large surface; future consolidation may be warranted.
- **Negative:** No ADR existed during implementation — architectural knowledge was encoded only in code and handoffs. This ADR rectifies that.

## Procedural Rhetoric

- **PS-01 (Shared Goal):** Media generation for agent communication and content creation.
- **PS-02 (Bounded Lexicon):** 6 tool categories, fal.ai backend, `media/{category}/{action}` naming.
- **PS-03 (Mode of Play):** Single-server, multi-category; fal.ai as primary backend.
- **PS-12 (Invitational Voice):** New media categories are invited via additional tool registrations.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
