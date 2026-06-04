#!/bin/bash
# Agent Pod Crate Generator for hKask
#
# This script generates a complete agent pod crate structure
# based on user responses to interactive prompts.
#
# Usage: ./generate-agent-pod.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     hKask Agent Pod Crate Generator v0.21.0           ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

# Prompt for agent information
echo -e "${YELLOW}=== Agent Identity ===${NC}"
read -p "Agent name (kebab-case, e.g., memory-curator-bot): " AGENT_NAME
read -p "Agent type (Bot/Replicant): " AGENT_TYPE
read -p "Editor/Administrator (e.g., curator, hKask-Administrator): " EDITOR
read -p "Agent description (1-2 sentences): " DESCRIPTION

echo ""
echo -e "${YELLOW}=== Workspace Configuration ===${NC}"
read -p "Workspace type (standalone/new/existing/bridge): " WORKSPACE_TYPE
read -p "Crate name (e.g., my-agent-crate): " CRATE_NAME
read -p "Parent workspace path (if existing, leave empty for new): " WORKSPACE_PATH

echo ""
echo -e "${YELLOW}=== Capabilities ===${NC}"
echo "Select capabilities (comma-separated):"
echo "  1. tool:inference:call"
echo "  2. tool:memory:recall"
echo "  3. tool:memory:remember"
echo "  4. tool:cns:emit"
echo "  5. tool:registry:index"
echo "  6. tool:template:render"
echo "  7. tool:ensemble:coordinate"
read -p "Capabilities (numbers): " CAP_SELECTION

# Convert capability numbers to strings
CAPABILITIES=""
if [[ $CAP_SELECTION == *"1"* ]]; then CAPABILITIES+="  - tool:inference:call\n"; fi
if [[ $CAP_SELECTION == *"2"* ]]; then CAPABILITIES+="  - tool:memory:recall\n"; fi
if [[ $CAP_SELECTION == *"3"* ]]; then CAPABILITIES+="  - tool:memory:remember\n"; fi
if [[ $CAP_SELECTION == *"4"* ]]; then CAPABILITIES+="  - tool:cns:emit\n"; fi
if [[ $CAP_SELECTION == *"5"* ]]; then CAPABILITIES+="  - tool:registry:index\n"; fi
if [[ $CAP_SELECTION == *"6"* ]]; then CAPABILITIES+="  - tool:template:render\n"; fi
if [[ $CAP_SELECTION == *"7"* ]]; then CAPABILITIES+="  - tool:ensemble:coordinate\n"; fi

echo ""
echo -e "${YELLOW}=== Visibility ===${NC}"
read -p "Default visibility (public/private/shared): " VISIBILITY
read -p "Episodic override (public/private/shared): " EPISODIC_OVERRIDE

echo ""
echo -e "${YELLOW}=== Generating crate structure...${NC}"

# Create directory structure
mkdir -p "$CRATE_NAME/templates/selectors"
mkdir -p "$CRATE_NAME/templates/prompts"
mkdir -p "$CRATE_NAME/templates/processes"
mkdir -p "$CRATE_NAME/templates/cognitions"

# Generate Cargo.toml
cat > "$CRATE_NAME/Cargo.toml" << EOF
[package]
name = "$CRATE_NAME"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Agent pod for $AGENT_NAME - $DESCRIPTION"

[dependencies]
hkask-types = { path = "../hkask-types" }
hkask-agents = { path = "../hkask-agents" }
hkask-templates = { path = "../hkask-templates" }
hkask-mcp = { path = "../hkask-mcp" }
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
hkask-testing = { path = "../hkask-testing" }
EOF

echo -e "${GREEN}✓ Created Cargo.toml${NC}"

# Generate agent_persona.yaml
cat > "$CRATE_NAME/agent_persona.yaml" << EOF
agent:
  name: $AGENT_NAME
  type: $AGENT_TYPE
  version: "0.1.0"
  binding_contract: true
  editor: $EDITOR

charter:
  description: >
    $DESCRIPTION
  archetype: Specialist
  visibility: Secondary

capabilities:
$(echo -e "$CAPABILITIES")

rights:
  - read: registry_index
  - read: template_catalog
  - execute: template_dispatch
  - write: own_episodic_memory

responsibilities:
  - respond_to: template_dispatch_requests
  - emit: cns.prompt.select
  - emit: cns.prompt.render
  - emit: cns.prompt.outcome
  - report_to: Curator
  - record: dispatch_operations_to_episodic_memory

visibility:
  default: "$VISIBILITY"
  episodic_override: "$EPISODIC_OVERRIDE"

reporting:
  receives_from: []
  report_to: standing_ensemble_session
  escalate_to: Curator
  escalation_triggers:
    - variety_deficit_gt_100
    - bot_coordination_failure
  report_interval: on_event_and_hourly_summary

standing_session:
  session_id: system-coordination-standing-session
  role: participant
  report_interval: hourly
  administrator_visible: true

process_manifest: registry/manifests/$AGENT_NAME-dispatch.yaml

depends_on:
  - hkask-mcp-inference
  - hkask-mcp-registry

readiness_probe:
  type: health_check
  endpoint: ${AGENT_NAME}::status
  expected:
    registry_index_available: true
    template_selector_ready: true
  timeout_seconds: 10
  retry_count: 3
EOF

echo -e "${GREEN}✓ Created agent_persona.yaml${NC}"

# Generate dispatch_manifest.yaml
cat > "$CRATE_NAME/dispatch_manifest.yaml" << EOF
# Dispatch manifest for $AGENT_NAME
# Routes template dispatch requests through selection → population → execution

manifest:
  name: $AGENT_NAME-dispatch
  version: "0.1.0"
  description: Template dispatch orchestration for $AGENT_NAME

matroshka:
  max_depth: 7
  enforce: true
  depth_counter:
    enabled: true
    inherit_from_parent: true
    default: 0
    increment_on_dispatch: true
  cns_monitoring:
    span_namespace: cns.prompt.matroshka_depth
    alert_if_exceeds: 6
    rationale: "Warning before hard limit at 7"

steps:
  - ordinal: 1
    action: select
    template_ref: registry/selectors/selector.j2
    renderer: minijinja
    model_tier: fast_local
    matroshka_depth: "\${matroshka_depth}"
    output_schema:
      type: object
      properties:
        selected_template_id:
          type: string
        rationale:
          type: string
        confidence:
          type: number
          minimum: 0.0
          maximum: 1.0

  - ordinal: 2
    action: populate
    template_ref: "\${selected_template_id}"
    renderer: minijinja
    bindings:
      raw_prompt: "\${input.raw_prompt}"
      context: "\${input.context}"
      matroshka_depth: "\${matroshka_depth + 1}"

  - ordinal: 3
    action: execute
    target: "\${template.contract.target}"
    contract: "\${template.contract}"
    mcp: "\${template.contract.mcp}"
    model_tier: "\${template.contract.model_tier}"
    matroshka_depth: "\${matroshka_depth + 1}"

cns:
  spans:
    - cns.prompt.select
    - cns.prompt.render
    - cns.prompt.outcome
EOF

echo -e "${GREEN}✓ Created dispatch_manifest.yaml${NC}"

# Generate selector template
cat > "$CRATE_NAME/templates/selectors/selector.j2" << 'EOF'
{# Template: selectors/selector.j2 #}
{# Purpose: Select best-fit template from registry for dispatch #}

You are a template selection engine for the hKask registry system.

Given the following available templates and the user's raw prompt, select the most appropriate template.

## Available Templates

{% for template in templates %}
### {{ template.id }}
- Type: {{ template.template_type }}
- Description: {{ template.description }}
- Lexicon terms: {{ template.lexicon_terms | join(', ') }}
- Contract: {{ template.contract | tojson }}

{% endfor %}

## User's Raw Prompt

{{ raw_prompt }}

## Selection Criteria

1. Match template_type to request nature:
   - Prompt (WordAct) → LLM/tool calls, speech acts
   - Process (FlowDef) → multi-step workflows, operations
   - Cognition (KnowAct) → thinking, learning, calibration

2. Match lexicon terms to prompt vocabulary

3. Consider contract compatibility (model_tier, mcp server)

## Output Format

Return JSON with:
- selected_template_id: ID of chosen template
- rationale: Why this template was selected
- confidence: Score 0.0 to 1.0

## Response

```json
{
  "selected_template_id": "...",
  "rationale": "...",
  "confidence": 0.0
}
```
EOF

echo -e "${GREEN}✓ Created templates/selectors/selector.j2${NC}"

# Generate prompt template
cat > "$CRATE_NAME/templates/prompts/prompt_render.j2" << 'EOF'
{# Template: prompts/prompt_render.j2 #}
{# Purpose: Render prompt for LLM inference #}

You are an AI assistant in the hKask agent platform.

## Context

{% if context %}
{{ context }}
{% endif %}

## Task

{{ raw_prompt }}

## Instructions

1. Analyze the request carefully
2. Apply relevant domain knowledge
3. Generate appropriate response
4. Follow output format specifications

## Output Format

[Specify expected output format based on task]

## Response

[Your response here]
EOF

echo -e "${GREEN}✓ Created templates/prompts/prompt_render.j2${NC}"

# Generate hlexicon.yaml
cat > "$CRATE_NAME/hlexicon.yaml" << EOF
# hLexicon terms for $AGENT_NAME
# Domain-specific vocabulary for template matching

- recognize
- classify
- match
- discriminate
- $AGENT_NAME
EOF

echo -e "${GREEN}✓ Created hlexicon.yaml${NC}"

# Create README.md
cat > "$CRATE_NAME/README.md" << EOF
# $AGENT_NAME

$DESCRIPTION

## Overview

This crate contains the agent pod for **$AGENT_NAME**, a $AGENT_TYPE in the hKask ecosystem.

## Structure

\`\`\`
$CRATE_NAME/
├── Cargo.toml              # Rust crate metadata
├── agent_persona.yaml      # Agent identity and capabilities
├── dispatch_manifest.yaml  # Dispatch workflow
├── hlexicon.yaml          # Domain terms
└── templates/
    ├── selectors/         # Template selectors
    ├── prompts/           # Prompt templates
    ├── processes/         # Process workflows
    └── cognitions/        # Cognition templates
\`\`\`

## Capabilities

$(echo -e "$CAPABILITIES" | sed 's/^  - /- /')

## Installation

\`\`\`bash
# Add to workspace Cargo.toml
[workspace]
members = [
    "$CRATE_NAME",
]

# Build
cargo build -p $CRATE_NAME
\`\`\`

## Registration

\`\`\`bash
# CLI registration
kask pod create \\
  --template $CRATE_NAME \\
  --persona agent_persona.yaml \\
  --name $AGENT_NAME

# Activate pod
kask pod activate <pod-id>
\`\`\`

## Monitoring

Monitor CNS spans:
- \`cns.prompt.select\` — Template selection events
- \`cns.prompt.render\` — Template rendering events
- \`cns.prompt.outcome\` — Execution results

## Dependencies

$(if [ -n "$WORKSPACE_PATH" ]; then echo "- Parent workspace: $WORKSPACE_PATH"; fi)
- hkask-mcp-inference
- hkask-mcp-registry

## License

MIT
EOF

echo -e "${GREEN}✓ Created README.md${NC}"

# Create workspace Cargo.toml if new workspace
if [ "$WORKSPACE_TYPE" == "new" ] || [ -z "$WORKSPACE_PATH" ]; then
    cat > "Cargo.toml" << EOF
[workspace]
members = [
    "$CRATE_NAME",
]
resolver = "2"

[workspace.dependencies]
serde = "1.0"
serde_json = "1.0"
serde_yaml = "0.9"
tokio = "1.0"
tracing = "0.1"
thiserror = "1.0"
EOF
    echo -e "${GREEN}✓ Created workspace Cargo.toml${NC}"
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Agent pod crate generated successfully!              ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Review and customize generated files"
echo "2. Add additional templates as needed"
echo "3. Build the crate: cargo build -p $CRATE_NAME"
echo "4. Register with ACP: kask pod create --template $CRATE_NAME --persona agent_persona.yaml --name $AGENT_NAME"
echo "5. Activate pod: kask pod activate <pod-id>"
echo ""
echo -e "${BLUE}For more information, see:${NC}"
echo "- docs/user-guides/AGENT-POD-CREATION-GUIDE.md"
echo "- docs/user-guides/AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md"
echo ""
