//! Logo window — renders the kask amphora with Curator's eye using ratatui Canvas.
//!
//! Maps `assets/kask-logo.svg` geometry (viewBox 0-400, 0-600) directly onto a
//! ratatui Canvas with Braille marker for highest terminal resolution.
//!
//! Elements rendered:
//!   - Bitemporal shadow (faint, offset left)
//!   - Rectangular amphora body with calligraphic stroke variation
//!   - Narrow neck with elliptical rim
//!   - Curved handles (quadratic bezier approximation)
//!   - Curator's eye: upper/lower eyelid, iris, pupil, reflection, authority gap
//!   - Eyelashes (subtle flicks)
//!   - "KASK" label (Paragraph below canvas)

use std::f64::consts::PI;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line as TextLine, Span, Text};
use ratatui::widgets::canvas::{Canvas, Circle, Context, Line, Rectangle};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::window::{Window, WindowId, WindowKind};

/// SVG viewBox dimensions.
const SVG_W: f64 = 400.0;
const SVG_H: f64 = 600.0;

/// A window that renders the hKask logo via ratatui Canvas.
pub struct LogoWindow {
    id: WindowId,
}

impl LogoWindow {
    pub fn new(id: WindowId) -> Self {
        Self { id }
    }
}

impl Window for LogoWindow {
    fn id(&self) -> WindowId {
        self.id
    }

    fn title(&self) -> &str {
        "Kask Logo"
    }

    fn kind(&self) -> WindowKind {
        WindowKind::Logo
    }

    fn render(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let block = Block::default()
            .title(" Kask Logo ")
            .borders(Borders::ALL)
            .style(Style::default().fg(border_color));

        let inner = block.inner(area);

        let label_h = 2u16;
        let canvas_h = inner.height.saturating_sub(label_h);
        let canvas_area = Rect::new(inner.x, inner.y, inner.width, canvas_h);
        let label_area = Rect::new(
            inner.x,
            inner.y.saturating_add(canvas_h),
            inner.width,
            label_h,
        );

        f.render_widget(block, area);

        let canvas = Canvas::default()
            .marker(Marker::Block)
            .x_bounds([0.0, SVG_W])
            .y_bounds([0.0, SVG_H])
            .paint(|ctx| draw_logo(ctx));

        f.render_widget(canvas, canvas_area);

        let label = Paragraph::new(Text::from(vec![
            TextLine::from(""),
            TextLine::from(vec![Span::styled(
                "K  A  S  K",
                Style::default().fg(Color::White),
            )]),
        ]))
        .alignment(Alignment::Center);

        f.render_widget(label, label_area);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
}

// ── Logo drawing ────────────────────────────────────────────────────

fn draw_logo(ctx: &mut Context) {
    let stroke = Color::White;
    let shadow = Color::DarkGray;

    draw_shadow(ctx, shadow);
    draw_body(ctx, stroke);
    draw_neck(ctx, stroke);
    draw_rim(ctx, stroke);
    draw_handles(ctx, stroke);
    draw_eye(ctx, stroke);
}

// ── Bitemporal shadow ───────────────────────────────────────────────

fn draw_shadow(ctx: &mut Context, color: Color) {
    // Shadow coords already offset -25px from main body in the SVG.
    // Body: x=145-285, Shadow: x=120-260 (-25px). No extra sx needed.
    ctx.draw(&Rectangle {
        x: 120.0,
        y: 185.0,
        width: 140.0,
        height: 255.0,
        color,
    });
    ctx.draw(&Rectangle {
        x: 150.0,
        y: 145.0,
        width: 80.0,
        height: 45.0,
        color,
    });
    ctx.draw(&Line::new(120.0, 200.0, 95.0, 235.0, color));
    ctx.draw(&Line::new(95.0, 235.0, 120.0, 265.0, color));
    ctx.draw(&Line::new(260.0, 200.0, 285.0, 235.0, color));
    ctx.draw(&Line::new(285.0, 235.0, 260.0, 265.0, color));
}

// ── Main amphora body ───────────────────────────────────────────────

fn draw_body(ctx: &mut Context, color: Color) {
    ctx.draw(&Line::new(145.0, 185.0, 145.0, 440.0, color));
    ctx.draw(&Line::new(143.0, 185.0, 143.0, 440.0, color));
    ctx.draw(&Line::new(285.0, 185.0, 285.0, 440.0, color));
    ctx.draw(&Line::new(287.0, 185.0, 287.0, 440.0, color));

    draw_sin_arc(ctx, 145.0, 440.0, 285.0, 5.0, color);
    draw_sin_arc(ctx, 145.0, 185.0, 285.0, -3.0, color);
}

// ── Neck ────────────────────────────────────────────────────────────

fn draw_neck(ctx: &mut Context, color: Color) {
    ctx.draw(&Line::new(170.0, 145.0, 170.0, 185.0, color));
    ctx.draw(&Line::new(168.0, 145.0, 168.0, 185.0, color));
    ctx.draw(&Line::new(260.0, 145.0, 260.0, 185.0, color));
    ctx.draw(&Line::new(262.0, 145.0, 262.0, 185.0, color));

    draw_sin_arc(ctx, 170.0, 145.0, 260.0, -3.0, color);
}

// ── Rim ─────────────────────────────────────────────────────────────

fn draw_rim(ctx: &mut Context, color: Color) {
    draw_sin_arc(ctx, 165.0, 145.0, 265.0, -8.0, color);
    draw_sin_arc(ctx, 165.0, 145.0, 265.0, 5.0, color);
}

// ── Handles ─────────────────────────────────────────────────────────

fn draw_handles(ctx: &mut Context, color: Color) {
    draw_bezier_handle(ctx, 145.0, 205.0, -35.0, color);
    draw_bezier_handle(ctx, 285.0, 205.0, 35.0, color);
}

fn draw_bezier_handle(ctx: &mut Context, x0: f64, y0: f64, extent: f64, color: Color) {
    let steps = 40;
    let mut prev_x = x0;
    let mut prev_y = y0;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let y = y0 + t * 60.0;
        let x = x0 + extent * (1.0 - (2.0 * t - 1.0).powi(2));
        ctx.draw(&Line::new(prev_x, prev_y, x, y, color));
        prev_x = x;
        prev_y = y;
    }
}

// ── Eye ─────────────────────────────────────────────────────────────

fn draw_eye(ctx: &mut Context, color: Color) {
    let cx = 215.0;
    let cy = 318.0;
    let eye_rx = 36.0;
    let eye_ry = 14.0;

    // 1. Eye white — filled horizontal oval (symmetric)
    draw_filled_ellipse(ctx, cx, cy, eye_rx, eye_ry, Color::Gray);

    // 2. Iris — filled circle, centered in eye
    draw_filled_circle(ctx, cx, cy, 11.0, Color::Black);

    // 3. Highlight — bright dot upper-right of iris
    draw_filled_circle(ctx, cx + 4.0, cy - 3.0, 3.0, Color::White);

    // 4. Eyebrow — arched curve above the eye
    for x in (cx as i32 - 34..=cx as i32 + 34).step_by(2) {
        let t = (x as f64 - (cx - 34.0)) / 68.0;
        let yy = cy - eye_ry - 4.0 - 5.0 * (PI * t).sin();
        ctx.draw(&Line::new(x as f64, yy, x as f64 + 1.0, yy, color));
        ctx.draw(&Line::new(
            x as f64,
            yy - 1.0,
            x as f64 + 1.0,
            yy - 1.0,
            color,
        ));
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Draw a sine-shaped arc suitable for body edges and rim.
fn draw_sin_arc(ctx: &mut Context, x1: f64, y: f64, x2: f64, dy: f64, color: Color) {
    let width = x2 - x1;
    for x in (x1 as i32..=x2 as i32).step_by(2) {
        let progress = (x as f64 - x1) / width;
        let yy = y + dy * (PI * progress).sin();
        ctx.draw(&Line::new(x as f64, yy, x as f64 + 1.0, yy, color));
    }
}

/// Draw an approximated filled ellipse by painting horizontal lines.
fn draw_filled_ellipse(ctx: &mut Context, cx: f64, cy: f64, rx: f64, ry: f64, color: Color) {
    let steps = (ry * 4.0).ceil() as i32;
    for i in 0..=steps {
        let y = cy - ry + (i as f64 * 2.0 * ry / steps as f64);
        let dy = (y - cy).abs();
        if dy <= ry {
            let dx = rx * (1.0 - (dy / ry).powi(2)).sqrt();
            ctx.draw(&Line::new(cx - dx, y, cx + dx, y, color));
        }
    }
}

/// Draw an approximated filled circle by painting horizontal lines.
fn draw_filled_circle(ctx: &mut Context, cx: f64, cy: f64, r: f64, color: Color) {
    let steps = (r * 4.0).ceil() as i32;
    for i in 0..=steps {
        let y = cy - r + (i as f64 * 2.0 * r / steps as f64);
        let dy = (y - cy).abs();
        if dy <= r {
            let dx = (r * r - dy * dy).sqrt();
            ctx.draw(&Line::new(cx - dx, y, cx + dx, y, color));
        }
    }
}
