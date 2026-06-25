---
title: "Writing Excellence Protocol"
audience: [contributors, developers, agents]
last_updated: 2026-06-14
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [curation]
---


# Writing Excellence Protocol

## 1. Purpose

This protocol defines the voice, style, and quality discipline for the hKask documentation corpus. It translates four independent dimensions of documentation quality — each grounded in the work of a woman who shaped technical communication — into a scoring rubric that governs publication decisions. It enforces the Writing Excellence Mandate in
[`DOCUMENTATION_STANDARDS.md`](../standards/DOCUMENTATION_STANDARDS.md) §1.[^schriver-readers]

## 2. Voice and Style Standards

These standards draw from plain-language technical writing principles established in research on reader cognition and document usability.[^schriver-dynamics]

### 2.1 Voice

| Dimension | Standard |
|-----------|----------|
| **Register** | Formal-technical. No slang, no hedging, no filler. |
| **Person** | Third person for specifications; second person for operator guides. Never first person plural ("we"). |
| **Tense** | Present tense for current-state descriptions; past tense only for historical provenance. |
| **Confidence** | Make assertions definite: "must", "shall", "does" — never "should probably", "might". State uncertainty as an explicit open question. |

### 2.2 Sentence Construction

- Maximum 35 words per sentence. Split if exceeded.
- One idea per sentence. One claim per paragraph.
- Active voice in all cases. Use passive voice only when describing what a source states.
- Define technical terms on first use unless in the project glossary or a preceding section.

### 2.3 Structural Discipline

Every section follows: **1. Statement** (what is true, in one sentence);
**2. Evidence** (code path, command, or external citation);
**3. Diagram** (visual rendering, if applicable);
**4. Implications** (what the reader should do with this knowledge).
A section without evidence is a draft. A section without implications is reference material.

### 2.4 Citation Density Requirements

| Document type | Minimum citations per `##` section |
|---------------|-----------------------------------|
| Architecture (A–D) | 1 external source |
| Specifications | 1 external source or 1 code-path verification |
| Standards | 1 external source |
| Operations | 0 (commands are self-verifying) |
| Plans | 0 (plans describe future work) |
| Status | 1 verification command per claim |

## 3. Quality Enforcement Process

Four tests assess independent dimensions via a multi-dimensional
scoring rubric.[^rubric]

| Score | Meaning | Publication Decision |
|-------|---------|---------------------|
| **1 of 4 passes** | Poor quality | Do not publish. Fundamental rework required. |
| **2 of 4 pass** | Passing | Acceptable for publication with noted gaps. |
| **3 of 4 pass** | Excellent | Publish confidently; remaining dimension is an improvement target. |
| **4 of 4 pass** | Exceptional | Rare. Use as a reference exemplar. |

**The goal is 3 of 4.** Different document types naturally emphasize
different dimensions; passing only 1 blocks publication.

### 3.1 Hopper Test (Accessibility)

Rear Admiral Grace Hopper — author of the first computer manual (1946),
inventor of FLOW-MATIC and the first compiler, lifelong advocate for
making machines speak human — demands: *Can a reader with zero prior
context accomplish the task by following only what is written?*[^hopper-yale][^hopper-britannica][^hopper-communicate]

| Operational Rule |
|------------------|
| Write for the reader's vocabulary. Agent-facing docs use MCP tool names and JSON schemas; operator docs use CLI paths. |
| If the audience cannot understand it, the writer has failed. Every README and `describe()` output must be comprehensible on first reading. |
| Build the bridge others called impossible. Concepts deemed "too complex to document" are precisely where documentation matters most. |

### 3.2 Lovelace Test (Precision)

Ada Lovelace — whose Notes on the Analytical Engine (1843) contained the
first published algorithm and described a non-existent machine with such
precision her specification remains verifiable 180 years later — demands:
*Could a reader write a correct implementation or test from this
specification alone?*[^lovelace-notes][^lovelace-babbage]

| Operational Rule |
|------------------|
| Document with enough precision that the specification is independently verifiable; a reader must be able to write a test from documentation alone. |
| See beyond immediate implementation. Articulate *why* a design exists, not merely *what* it does. |
| Annotate with more depth than the original. An ADR's context and consequences must exceed its decision statement. |

### 3.3 Schriver Test (Findability)

Karen Schriver — whose *Dynamics in Document Design* (1997) proved
quality is measurable by reader outcomes, not author intent — demands:
*Can a reader find the answer to their specific question within 30 seconds
of arriving at this document?*[^schriver-dynamics][^schriver-attw]

| Operational Rule |
|------------------|
| Design for how readers actually read. Every document must have scannable headings, a navigation table, and diagrams at the point of use. |
| Integrate word and image as a single communication unit. Prose must reference diagrams explicitly; neither should be comprehensible alone. |
| Measure quality by reader outcomes. If the answer is not findable in 30 seconds, the document has failed. |

### 3.4 Gentle Test (Agent-Correctness)

Anne Gentle — whose *Docs Like Code* (2017) codified that documentation
shares code's lifecycle, CI, review, and contributor workflows, and who
led OpenStack's 130-repo community documentation at scale — demands:
*If an AI agent consumed this document as its sole source of truth about
the system, would it behave correctly?* In an agent-native system, stale
documentation is a functional defect that produces incorrect agent behavior.[^gentle-docs][^gentle-openstack][^gentle-about]

| Operational Rule |
|------------------|
| Documentation lives in the same repo and shares the same review process as code. Doc and code changes for the same feature belong in the same commit. |
| Automate quality gates — link checking, stale-name detection, diagram metadata — rather than relying on human vigilance. |
| Use developer-native tooling (Markdown, git, standard CLI). Anyone who writes `cargo test` can write a documentation section. |
| Broken docs block the build. A stale package count carries the same severity as a compilation error. |

## References

[^hopper-yale]: Office of the President, Yale University. (2017). *Biography of Grace Murray Hopper*. <https://president.yale.edu/biography-grace-murray-hopper>.
[^hopper-britannica]: Britannica, T. Editors. (2024). *Grace Hopper*. <https://www.britannica.com/biography/Grace-Hopper>.
[^hopper-communicate]: Hopper, G. M. (1980), as quoted in Beyer, K. W. (2009). *Grace Hopper and the Invention of the Information Age*. MIT Press.
[^lovelace-notes]: Lovelace, A. A. (1843). Notes on Menabrea's "Sketch of the Analytical Engine." *Scientific Memoirs*, 3.
[^lovelace-babbage]: Babbage, C. (1864). *Passages from the Life of a Philosopher*. Longman, Green.
[^schriver-dynamics]: Schriver, K. A. (1997). *Dynamics in Document Design: Creating Text for Readers*. Wiley.
[^schriver-readers]: Schriver, K. A. (2012). What do technical communicators need to know about information design? In Johnson-Eilola & Selber (Eds.), *Solving Problems in Technical Communication*. U. Chicago Press.
[^schriver-attw]: Association of Teachers of Technical Writing. (2015). *2015 ATTW Fellow: Karen Schriver*. <https://attw.org/about-attw/attw-fellows/2015-karen-schriver/>.
[^rubric]: Stevens, D. D., & Levi, A. J. (2013). *Introduction to Rubrics* (2nd ed.). Stylus Publishing.
[^gentle-docs]: Gentle, A. (2017). *Docs Like Code: Collaborate and Automate to Improve Technical Documentation*. Just Write Click. <https://www.docslikecode.com/book/>.
[^gentle-openstack]: Gentle, A. (2016). Git and GitHub for open source documentation. *OpenSource.com*. <https://opensource.com/article/16/4/git-and-github-open-source-documentation>.
[^gentle-about]: Gentle, A. (2024). About Docs Like Code. <https://www.docslikecode.com/about/>.
[^matc-women]: Bogue, M. (2025). Women Who Shaped Technical Writing. MATC Group. <https://www.matcgroup.com/technical-writing/women-who-shaped-technical-writing-a-history-of-progress-struggles-and-successes/>.