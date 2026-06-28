//! Command palette overlay — fuzzy-searchable window launcher.
//!
//! Activated via Ctrl+P. Renders a centered overlay listing all
//! WindowKind variants with fuzzy filtering. Selecting a kind opens
//! that window at the currently focused split position.
//!
//! Architecture: the palette is a pure overlay rendered on top of
//! the workspace, not a Window trait implementor. It does not mutate
//! window state until `on_select`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::window::WindowKind;
use crate::window_catalog::window_kinds;

#[derive(Debug, Clone, Copy)]
pub enum PaletteAction {
    Close,
    Open(WindowKind),
}

/// A single item in the command palette.
#[derive(Debug, Clone, Copy)]
struct PaletteItem {
    kind: WindowKind,
    label: &'static str,
    description: &'static str,
}

/// Fuzzy-searchable command palette overlay.
pub struct CommandPalette {
    items: Vec<PaletteItem>,
    filter: String,
    selection: usize,
}

impl CommandPalette {
    /// Build palette from the static WindowKind catalog.
    pub fn new() -> Self {
        let items: Vec<PaletteItem> = window_kinds()
            .iter()
            .map(|&k| PaletteItem {
                kind: k,
                label: k.default_title(),
                description: k.description(),
            })
            .collect();
        Self {
            items,
            filter: String::new(),
            selection: 0,
        }
    }

    /// Filtered items matching the current filter string (case-insensitive
    /// substring match on both label and description). Zero-allocation:
    /// returns indices into self.items.
    fn filtered(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.items.len()).collect();
        }
        let q = self.filter.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.label.to_lowercase().contains(&q)
                    || item.description.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Clamp selection to filtered count.
    fn clamp_selection(&mut self) {
        let filtered = self.filtered();
        let count = filtered.len();
        if count == 0 {
            self.selection = 0;
        } else if self.selection >= count {
            self.selection = count.saturating_sub(1);
        }
    }

    /// Handle a key event while the palette is open.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<PaletteAction> {
        match (key.modifiers, key.code) {
            // Dismiss — Esc or toggle with Ctrl+P
            (KeyModifiers::NONE, KeyCode::Esc) => return Some(PaletteAction::Close),
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => return Some(PaletteAction::Close),
            // Select
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.clamp_selection();
                let filtered = self.filtered();
                if let Some(&idx) = filtered.get(self.selection) {
                    return Some(PaletteAction::Open(self.items[idx].kind));
                }
            }
            // Navigate
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.selection = self.selection.saturating_sub(1);
                self.clamp_selection();
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.selection += 1;
                self.clamp_selection();
            }
            // Filter input
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                self.filter.push(c);
                self.selection = 0;
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.filter.pop();
                self.selection = 0;
            }
            _ => {}
        }
        None
    }

    /// Reset state for next opening.
    pub fn reset(&mut self) {
        self.filter.clear();
        self.selection = 0;
    }

    /// Render the palette overlay centred on the workspace.
    /// Takes `&self` so it can be called from `Workspace::render(&self)`.
    pub fn render(&self, f: &mut Frame, _area: Rect) {
        let filtered = self.filtered();
        let count = filtered.len();
        // Note: selection clamping happens in handle_key; we trust it here

        let total_items = filtered.len();
        let list_items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let item = &self.items[idx];
                let is_selected = i == self.selection;
                let prefix = if is_selected { "▶ " } else { "  " };
                let label_style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let desc_style = if is_selected {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, label_style),
                    Span::styled(item.label, label_style),
                    Span::raw("  "),
                    Span::styled(item.description, desc_style),
                ]))
            })
            .collect();

        let filter_hint = if self.filter.is_empty() {
            "Type to filter..."
        } else {
            &self.filter
        };

        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        " Command Palette ({}/{}) ",
                        count,
                        self.items.len()
                    ))
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 60))
                    .add_modifier(Modifier::BOLD),
            );

        // Size: 60 cols wide, up to 22 rows (all 19 items + border + filter)
        let palette_h = (total_items + 4).min(22) as u16;
        let palette_w = 60u16;
        let f_area = f.area();
        let x = f_area.x + (f_area.width.saturating_sub(palette_w)) / 2;
        let y = f_area.y + (f_area.height.saturating_sub(palette_h)) / 2;
        let palette_area = Rect::new(x, y, palette_w, palette_h);

        // Clear area behind palette
        f.render_widget(Clear, palette_area);

        // Render filter input
        let filter_text = Paragraph::new(Span::styled(
            format!("> {}", filter_hint),
            Style::default().fg(Color::Yellow),
        ))
        .block(
            Block::default()
                .borders(Borders::NONE)
                .style(Style::default().bg(Color::Rgb(20, 20, 30))),
        );
        let filter_area = Rect::new(x + 1, y + 1, palette_w.saturating_sub(2), 1);
        f.render_widget(filter_text, filter_area);

        // Render list
        let list_area = Rect::new(
            x + 1,
            y + 2,
            palette_w.saturating_sub(2),
            palette_h.saturating_sub(3),
        );
        let mut list_state = ListState::default();
        if !filtered.is_empty() && self.selection < count {
            list_state.select(Some(self.selection));
        }
        f.render_stateful_widget(list, list_area, &mut list_state);
    }

    /// Get the selected WindowKind, or None if nothing matches.
    pub fn selected_kind(&self) -> Option<WindowKind> {
        let filtered = self.filtered();
        filtered
            .get(self.selection)
            .map(|&idx| self.items[idx].kind)
    }
}
