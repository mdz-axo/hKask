//! Memory window — view and edit agent memories.
//!
//! Shows episodic (private) and semantic (public) memories for the
//! current agent. Supports browsing, searching, and editing triples.
//! Integrates with hkask-memory consolidation service.
//!
//! # Architecture
//! ⟨Memory⟩ displays ⟨EpisodicMemory, SemanticMemory, Triples⟩ .
//! ⟨Memory⟩ integratesWith ⟨hkask-memory, ConsolidationService⟩ .

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemorySection {
    Episodic,
    Semantic,
    Triples,
    Consolidation,
}

impl MemorySection {
    fn next(self) -> Self {
        match self {
            Self::Episodic => Self::Semantic,
            Self::Semantic => Self::Triples,
            Self::Triples => Self::Consolidation,
            Self::Consolidation => Self::Episodic,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Episodic => "Episodic",
            Self::Semantic => "Semantic",
            Self::Triples => "Triples",
            Self::Consolidation => "Consolidation",
        }
    }
}

pub struct MemoryWindow {
    id: WindowId,
    section: MemorySection,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
}

impl MemoryWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: MemorySection::Episodic,
            bridge,
        }
    }
}

impl Window for MemoryWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Memory"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Memory
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Memory: {} (Tab to switch) ──", self.section.title()),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];
        match self.section {
            MemorySection::Episodic => {
                lines.push(Line::from(
                    "  Episodic memory — private, agent-scoped experiences.",
                ));
                lines.push(Line::from("  Each tool call and interaction is recorded."));
                lines.push(Line::from("  Query via perspective = agent_webid (P11)."));
                lines.push(Line::from(""));
                lines.push(Line::from("  Stored in per-pod SQLCipher DB (P11.1)."));
            }
            MemorySection::Semantic => {
                lines.push(Line::from("  Semantic memory — shared, public knowledge."));
                lines.push(Line::from("  Built from consolidated episodic triples."));
                lines.push(Line::from(
                    "  CuratorPod SemanticIndex aggregates all pods.",
                ));
                lines.push(Line::from(""));
                lines.push(Line::from(
                    "  Use /consolidate to trigger episodic→semantic.",
                ));
            }
            MemorySection::Triples => {
                lines.push(Line::from(
                    "  RDF triples: ⟨subject⟩ ⟨predicate⟩ ⟨object⟩ .",
                ));
                lines.push(Line::from("  Each triple has:"));
                lines.push(Line::from("    • entity, attribute, value"));
                lines.push(Line::from("    • confidence (0.0–1.0)"));
                lines.push(Line::from("    • visibility (Private/Public)"));
                lines.push(Line::from("    • owner WebID (P12)"));
            }
            MemorySection::Consolidation => {
                lines.push(Line::from(
                    "  Consolidation: episodic → semantic every N experiences.",
                ));
                lines.push(Line::from(
                    "  Confidence floor: 0.33 (low-confidence triples removed).",
                ));
                lines.push(Line::from("  Max semantic triples configurable."));
                lines.push(Line::from(""));
                lines.push(Line::from("  Use /consolidate to trigger manually."));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Memory model: ν-events → episodic (private) → semantic (public) → SemanticIndex",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            self.section = self.section.next();
            true
        } else {
            false
        }
    }
    fn tick(&mut self) {}
}
