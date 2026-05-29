---
title: "Prototype and Demonstration Artifacts"
audience: [developers, template authors]
last_updated: 2026-05-28
version: "0.21.0"
status: "Active"
domain: "Application"
ddmvss_categories: [composition]
---

# Prototype and Demonstration Artifacts

Files in this directory are **prototype and demonstration templates only**.
They are **not registered in the live template system** and must not be added
to `Registry::bootstrap()`.

These artifacts exist to illustrate template structure, frontmatter conventions,
and hLexicon integration patterns. They are not loadable by the template
renderer at runtime and are excluded from all tool-discovery surfaces.

**Do not** add files from this directory to the registry. If a prototype
becomes production-ready, copy it to the appropriate `registry/templates/<domain>/`
directory with a `.j2` extension and register it in `Registry::bootstrap()`.