//! Memory window — browse and view agent memories.
//!
//! Shows episodic (private) and semantic (shared) memories, triples,
//! and consolidation status. Tab-cycled sections: Episodic, Semantic,
//! Triples, Consolidation.
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

use crate::bridges::MemoryDataBridge;
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
    memory: Option<Arc<dyn MemoryDataBridge>>,
}

impl MemoryWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: MemorySection::Episodic,
            bridge,
            memory: None,
        }
    }

    pub fn with_memory_bridge(mut self, mem: Arc<dyn MemoryDataBridge>) -> Self {
        self.memory = Some(mem);
        self
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

        if let Some(ref mem) = self.memory {
            let summary = mem.memory_summary();
            match self.section {
                MemorySection::Episodic => {
                    lines.push(Line::from(format!(
                        "  Episodic usage: {} / {} ({:.0}%)",
                        summary.episodic_count,
                        summary.episodic_budget,
                        if summary.episodic_budget > 0 {
                            (summary.episodic_count as f64 / summary.episodic_budget as f64) * 100.0
                        } else {
                            0.0
                        }
                    )));
                    lines.push(Line::from(""));
                    let triples = mem.recent_episodic(15);
                    if triples.is_empty() {
                        lines.push(Line::from("  No episodic memories recorded yet."));
                    } else {
                        for t in &triples {
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(
                                    format!("{}", t.entity),
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::raw(" · "),
                                Span::styled(
                                    format!("{}", t.attribute),
                                    Style::default().fg(Color::Yellow),
                                ),
                                Span::raw(" = "),
                                Span::styled(
                                    format!("{}", t.value),
                                    Style::default().fg(Color::White),
                                ),
                            ]));
                        }
                    }
                }
                MemorySection::Semantic => {
                    lines.push(Line::from(format!(
                        "  Semantic triples: {}",
                        summary.semantic_count
                    )));
                    lines.push(Line::from(format!(
                        "  Low confidence (≤0.33): {}",
                        summary.semantic_low_confidence
                    )));
                    lines.push(Line::from(""));
                    let triples = mem.recent_semantic(15);
                    if triples.is_empty() {
                        lines.push(Line::from("  No semantic triples stored."));
                    } else {
                        for t in &triples {
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(
                                    format!("{}", t.entity),
                                    Style::default().fg(Color::Green),
                                ),
                                Span::raw(" · "),
                                Span::styled(
                                    format!("{}", t.attribute),
                                    Style::default().fg(Color::Yellow),
                                ),
                                Span::raw(" = "),
                                Span::styled(
                                    format!("{}", t.value),
                                    Style::default().fg(Color::White),
                                ),
                            ]));
                        }
                    }
                }
                MemorySection::Triples => {
                    lines.push(Line::from(
                        "  RDF triples: ⟨subject⟩ ⟨predicate⟩ ⟨object⟩ .",
                    ));
                    lines.push(Line::from(format!(
                        "  Episodic: {}   Semantic: {}",
                        summary.episodic_count, summary.semantic_count
                    )));
                    lines.push(Line::from("  Each triple stores:"));
                    lines.push(Line::from("    • entity, attribute, value"));
                    lines.push(Line::from("    • confidence (0.0–1.0)"));
                    lines.push(Line::from("    • visibility (Private/Public)"));
                    lines.push(Line::from("    • owner WebID (P12)"));
                }
                MemorySection::Consolidation => {
                    let cs = mem.consolidation_status();
                    lines.push(Line::from(format!(
                        "  Consolidation candidates: {}",
                        cs.candidate_count
                    )));
                    lines.push(Line::from(format!(
                        "  Semantic total: {}",
                        cs.semantic_count
                    )));
                    lines.push(Line::from(format!(
                        "  Low-confidence: {} (≤0.33 floor)",
                        cs.low_confidence_count
                    )));
                    lines.push(Line::from(format!(
                        "  Episodic budget: {}",
                        cs.episodic_budget
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(
                        "  Consolidation: episodic → semantic every N experiences.",
                    ));
                    lines.push(Line::from(
                        "  Confidence floor: 0.33 (low-confidence triples removed).",
                    ));
                    lines.push(Line::from("  Use /consolidate to trigger manually."));
                }
            }
        } else {
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
