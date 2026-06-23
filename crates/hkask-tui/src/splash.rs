//! Splash screen and persistent logo rendering.
//!
//! Rasterizes the Kask SVG geometry (`assets/kask-logo.svg`) into pixel buffers
//! and renders using Unicode half-block characters (`▀ ▄ █`). Supports two sizes:
//!
//! - **Full splash** (scale 0.2, 80×60 chars) — centered full-screen on launch
//! - **Logo window** (scale 0.1, 40×30 chars) — persistent top-left corner
//!
//! Pixel values: 0=bg, 1=main stroke, 2=shadow, 3=highlight

use crossterm::event::{self, Event};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::time::{Duration, Instant};

const SPLASH_DURATION_MS: u64 = 2500;

// ── Rasterization context ────────────────────────────────────────────

struct LogoCanvas {
    buf: Vec<u8>,
    w: usize,
    h: usize,
    scale: f64,
}

impl LogoCanvas {
    fn new(scale: f64) -> Self {
        let w = (400.0 * scale) as usize;
        let h = (600.0 * scale) as usize;
        Self {
            buf: vec![0u8; w * h],
            w,
            h,
            scale,
        }
    }

    fn sx(&self, x: f64) -> f64 {
        x * self.scale
    }
    fn sy(&self, y: f64) -> f64 {
        y * self.scale
    }

    fn set_pixel(&mut self, x: f64, y: f64, value: u8) {
        let ix = x.round() as i32;
        let iy = y.round() as i32;
        if ix >= 0 && (ix as usize) < self.w && iy >= 0 && (iy as usize) < self.h {
            let idx = iy as usize * self.w + ix as usize;
            if value == 1 || value == 3 || self.buf[idx] == 0 {
                self.buf[idx] = value;
            }
        }
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, value: u8) {
        let ix0 = x0.round() as i32;
        let iy0 = y0.round() as i32;
        let ix1 = x1.round() as i32;
        let iy1 = y1.round() as i32;
        let dx = (ix1 - ix0).abs();
        let dy = -(iy1 - iy0).abs();
        let sx: i32 = if ix0 < ix1 { 1 } else { -1 };
        let sy: i32 = if iy0 < iy1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = ix0;
        let mut y = iy0;
        loop {
            if x >= 0 && (x as usize) < self.w && y >= 0 && (y as usize) < self.h {
                let idx = y as usize * self.w + x as usize;
                if value == 1 || value == 3 || self.buf[idx] == 0 {
                    self.buf[idx] = value;
                }
            }
            if x == ix1 && y == iy1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn thick_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, width: i32, value: u8) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            self.set_pixel(x0, y0, value);
            return;
        }
        let nx = -dy / len;
        let ny = dx / len;
        let half_w = (width as f64) / 2.0;
        for w in 0..width {
            let offset = (w as f64) - half_w + 0.5;
            self.line(
                x0 + nx * offset,
                y0 + ny * offset,
                x1 + nx * offset,
                y1 + ny * offset,
                value,
            );
        }
    }

    fn quad_bezier(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        width: i32,
        value: u8,
    ) {
        let steps = 40;
        let mut px = x0;
        let mut py = y0;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let mt = 1.0 - t;
            let qx = mt * mt * x0 + 2.0 * mt * t * x1 + t * t * x2;
            let qy = mt * mt * y0 + 2.0 * mt * t * y1 + t * t * y2;
            self.thick_line(px, py, qx, qy, width, value);
            px = qx;
            py = qy;
        }
    }

    fn cubic_bezier(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        width: i32,
        value: u8,
    ) {
        let steps = 50;
        let mut px = x0;
        let mut py = y0;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let mt = 1.0 - t;
            let cx =
                mt * mt * mt * x0 + 3.0 * mt * mt * t * x1 + 3.0 * mt * t * t * x2 + t * t * t * x3;
            let cy =
                mt * mt * mt * y0 + 3.0 * mt * mt * t * y1 + 3.0 * mt * t * t * y2 + t * t * t * y3;
            self.thick_line(px, py, cx, cy, width, value);
            px = cx;
            py = cy;
        }
    }

    fn filled_circle(&mut self, cx: f64, cy: f64, r: f64, value: u8) {
        let ri = r.round() as i32;
        let cxi = cx.round() as i32;
        let cyi = cy.round() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                if dx * dx + dy * dy <= ri * ri {
                    let px = cxi + dx;
                    let py = cyi + dy;
                    if px >= 0 && (px as usize) < self.w && py >= 0 && (py as usize) < self.h {
                        let idx = py as usize * self.w + px as usize;
                        if value == 1 || value == 3 || self.buf[idx] == 0 {
                            self.buf[idx] = value;
                        }
                    }
                }
            }
        }
    }

    fn rect_stroke(&mut self, x: f64, y: f64, w: f64, h: f64, width: i32, value: u8) {
        self.thick_line(x, y, x + w, y, width, value);
        self.thick_line(x, y + h, x + w, y + h, width, value);
        self.thick_line(x, y, x, y + h, width, value);
        self.thick_line(x + w, y, x + w, y + h, width, value);
    }
}

// ── Logo geometry — drawn into any LogoCanvas ───────────────────────

fn draw_logo_geometry(c: &mut LogoCanvas) {
    let so = -25.0; // bitemporal shadow offset

    // Bitemporal shadow
    c.thick_line(
        c.sx(160.0 + so),
        c.sy(180.0),
        c.sx(140.0 + so),
        c.sy(440.0),
        1,
        2,
    );
    c.thick_line(
        c.sx(140.0 + so),
        c.sy(440.0),
        c.sx(140.0 + so),
        c.sy(460.0),
        1,
        2,
    );
    c.thick_line(
        c.sx(140.0 + so),
        c.sy(460.0),
        c.sx(200.0 + so),
        c.sy(460.0),
        1,
        2,
    );
    c.thick_line(
        c.sx(200.0 + so),
        c.sy(460.0),
        c.sx(260.0 + so),
        c.sy(460.0),
        1,
        2,
    );
    c.thick_line(
        c.sx(260.0 + so),
        c.sy(460.0),
        c.sx(260.0 + so),
        c.sy(440.0),
        1,
        2,
    );
    c.thick_line(
        c.sx(260.0 + so),
        c.sy(440.0),
        c.sx(240.0 + so),
        c.sy(180.0),
        1,
        2,
    );
    c.thick_line(
        c.sx(240.0 + so),
        c.sy(180.0),
        c.sx(160.0 + so),
        c.sy(180.0),
        1,
        2,
    );
    c.rect_stroke(c.sx(170.0 + so), c.sy(145.0), c.sx(60.0), c.sy(40.0), 1, 2);
    c.cubic_bezier(
        c.sx(165.0 + so),
        c.sy(160.0),
        c.sx(165.0 + so),
        c.sy(80.0),
        c.sx(200.0 + so),
        c.sy(60.0),
        c.sx(200.0 + so),
        c.sy(60.0),
        1,
        2,
    );
    c.cubic_bezier(
        c.sx(200.0 + so),
        c.sy(60.0),
        c.sx(200.0 + so),
        c.sy(60.0),
        c.sx(235.0 + so),
        c.sy(80.0),
        c.sx(235.0 + so),
        c.sy(160.0),
        1,
        2,
    );

    // Main jug body
    c.thick_line(c.sx(160.0), c.sy(180.0), c.sx(140.0), c.sy(440.0), 2, 1);
    c.thick_line(c.sx(240.0), c.sy(180.0), c.sx(260.0), c.sy(440.0), 2, 1);
    c.quad_bezier(
        c.sx(140.0),
        c.sy(440.0),
        c.sx(140.0),
        c.sy(460.0),
        c.sx(200.0),
        c.sy(460.0),
        2,
        1,
    );
    c.quad_bezier(
        c.sx(200.0),
        c.sy(460.0),
        c.sx(260.0),
        c.sy(460.0),
        c.sx(260.0),
        c.sy(440.0),
        2,
        1,
    );
    c.quad_bezier(
        c.sx(160.0),
        c.sy(180.0),
        c.sx(200.0),
        c.sy(175.0),
        c.sx(240.0),
        c.sy(180.0),
        1,
        1,
    );

    // Ribbing
    c.quad_bezier(
        c.sx(155.0),
        c.sy(240.0),
        c.sx(200.0),
        c.sy(238.0),
        c.sx(245.0),
        c.sy(240.0),
        1,
        1,
    );
    c.quad_bezier(
        c.sx(150.0),
        c.sy(320.0),
        c.sx(200.0),
        c.sy(318.0),
        c.sx(250.0),
        c.sy(320.0),
        1,
        1,
    );
    c.quad_bezier(
        c.sx(145.0),
        c.sy(400.0),
        c.sx(200.0),
        c.sy(398.0),
        c.sx(255.0),
        c.sy(400.0),
        1,
        1,
    );

    // Neck
    c.thick_line(c.sx(170.0), c.sy(145.0), c.sx(170.0), c.sy(185.0), 2, 1);
    c.thick_line(c.sx(230.0), c.sy(145.0), c.sx(230.0), c.sy(185.0), 2, 1);
    c.quad_bezier(
        c.sx(170.0),
        c.sy(145.0),
        c.sx(200.0),
        c.sy(140.0),
        c.sx(230.0),
        c.sy(145.0),
        1,
        1,
    );

    // Rim
    c.cubic_bezier(
        c.sx(165.0),
        c.sy(145.0),
        c.sx(165.0),
        c.sy(138.0),
        c.sx(200.0),
        c.sy(135.0),
        c.sx(235.0),
        c.sy(145.0),
        1,
        1,
    );
    c.cubic_bezier(
        c.sx(235.0),
        c.sy(145.0),
        c.sx(235.0),
        c.sy(152.0),
        c.sx(200.0),
        c.sy(155.0),
        c.sx(165.0),
        c.sy(145.0),
        1,
        1,
    );

    // Spout
    c.cubic_bezier(
        c.sx(165.0),
        c.sy(145.0),
        c.sx(155.0),
        c.sy(140.0),
        c.sx(150.0),
        c.sy(135.0),
        c.sx(145.0),
        c.sy(130.0),
        1,
        1,
    );

    // Bail handle
    c.filled_circle(c.sx(165.0), c.sy(160.0), c.sx(4.0), 1);
    c.filled_circle(c.sx(235.0), c.sy(160.0), c.sx(4.0), 1);
    c.cubic_bezier(
        c.sx(165.0),
        c.sy(160.0),
        c.sx(165.0),
        c.sy(80.0),
        c.sx(200.0),
        c.sy(60.0),
        c.sx(200.0),
        c.sy(60.0),
        1,
        1,
    );
    c.cubic_bezier(
        c.sx(200.0),
        c.sy(60.0),
        c.sx(200.0),
        c.sy(60.0),
        c.sx(235.0),
        c.sy(80.0),
        c.sx(235.0),
        c.sy(160.0),
        1,
        1,
    );
    c.quad_bezier(
        c.sx(190.0),
        c.sy(65.0),
        c.sx(200.0),
        c.sy(60.0),
        c.sx(210.0),
        c.sy(65.0),
        2,
        1,
    );

    // Curator's Eye
    c.quad_bezier(
        c.sx(175.0),
        c.sy(310.0),
        c.sx(200.0),
        c.sy(295.0),
        c.sx(225.0),
        c.sy(310.0),
        2,
        1,
    );
    c.quad_bezier(
        c.sx(178.0),
        c.sy(330.0),
        c.sx(200.0),
        c.sy(350.0),
        c.sx(222.0),
        c.sy(330.0),
        1,
        1,
    );
    c.filled_circle(c.sx(200.0), c.sy(320.0), c.sx(18.0), 1);
    c.filled_circle(c.sx(200.0), c.sy(320.0), c.sx(10.0), 1);
    c.filled_circle(c.sx(205.0), c.sy(315.0), c.sx(5.0), 3);
    c.thick_line(c.sx(188.0), c.sy(308.0), c.sx(185.0), c.sy(302.0), 1, 1);
    c.thick_line(c.sx(196.0), c.sy(305.0), c.sx(194.0), c.sy(298.0), 1, 1);
    c.thick_line(c.sx(204.0), c.sy(305.0), c.sx(206.0), c.sy(298.0), 1, 1);
    c.thick_line(c.sx(212.0), c.sy(308.0), c.sx(215.0), c.sy(302.0), 1, 1);
    c.thick_line(c.sx(208.0), c.sy(312.0), c.sx(210.0), c.sy(328.0), 1, 3);
}

fn build_logo_buffer(scale: f64) -> (Vec<u8>, usize, usize) {
    let mut c = LogoCanvas::new(scale);
    draw_logo_geometry(&mut c);
    (c.buf, c.w, c.h)
}

fn splash_buffer() -> (Vec<u8>, usize, usize) {
    build_logo_buffer(0.2) // 80×120 px → 80×60 chars
}

fn logo_window_buffer() -> (Vec<u8>, usize, usize) {
    build_logo_buffer(0.1) // 40×60 px → 40×30 chars
}

// ── Half-block pixel mapping ───────────────────────────────────────

fn half_block_pixel(top: u8, bot: u8) -> (&'static str, Color, Color) {
    let bg = Color::Rgb(11, 12, 21);
    let main = Color::Rgb(224, 224, 224);
    let shadow = Color::Rgb(60, 60, 70);
    let highlight = Color::White;
    let top_color = pixel_color(top, main, shadow, highlight, bg);
    let bot_color = pixel_color(bot, main, shadow, highlight, bg);
    match (top, bot) {
        (0, 0) => (" ", bg, bg),
        (0, _) => ("▄", bot_color, bg),
        (_, 0) => ("▀", top_color, bg),
        _ if top == bot => ("█", top_color, bg),
        _ => ("▀", top_color, bot_color),
    }
}

fn pixel_color(value: u8, main: Color, shadow: Color, highlight: Color, bg: Color) -> Color {
    match value {
        1 => main,
        2 => shadow,
        3 => highlight,
        _ => bg,
    }
}

/// Render a pixel buffer as half-block character rows into a Vec<Line>.
fn render_logo_lines(
    buf: &[u8],
    w: usize,
    h: usize,
    show_wordmark: bool,
    show_prompt: bool,
) -> Vec<Line<'static>> {
    let term_cols = w;
    let term_rows = h / 2;
    let mut lines: Vec<Line> = Vec::new();

    for row in 0..term_rows {
        let mut spans: Vec<Span> = Vec::new();
        let top_y = row * 2;
        let bot_y = row * 2 + 1;
        for col in 0..term_cols {
            let top = buf[top_y * w + col];
            let bot = buf[bot_y * w + col];
            let (ch, fg, bg_color) = half_block_pixel(top, bot);
            spans.push(Span::styled(ch, Style::default().fg(fg).bg(bg_color)));
        }
        lines.push(Line::from(spans));
    }

    if show_wordmark {
        lines.push(Line::from(""));
        let label = if term_cols >= 40 {
            "K  A  S  K"
        } else {
            "KASK"
        };
        lines.push(Line::from(Span::styled(
            format!(
                "{:>width$}",
                label,
                width = term_cols / 2 + label.len() / 2 + 1
            ),
            Style::default()
                .fg(Color::Rgb(224, 224, 224))
                .bg(Color::Rgb(11, 12, 21)),
        )));
    }

    if show_prompt {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "{:>width$}",
                "Press any key to continue",
                width = term_cols / 2 + 14
            ),
            Style::default()
                .fg(Color::DarkGray)
                .bg(Color::Rgb(11, 12, 21)),
        )));
    }

    lines
}

// ── Splash screen widget ────────────────────────────────────────────

pub struct SplashScreen {
    start_time: Instant,
    duration: Duration,
    buffer: Vec<u8>,
    buf_w: usize,
    buf_h: usize,
}

impl SplashScreen {
    pub fn new() -> Self {
        let (buf, w, h) = splash_buffer();
        Self {
            start_time: Instant::now(),
            duration: Duration::from_millis(SPLASH_DURATION_MS),
            buffer: buf,
            buf_w: w,
            buf_h: h,
        }
    }

    pub fn should_dismiss(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }

    pub fn check_early_dismiss(&mut self) -> bool {
        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            if let Ok(Event::Key(_)) = event::read() {
                return true;
            }
        }
        false
    }

    pub fn render(&self, f: &mut Frame) {
        let area = f.area();
        let bg_block =
            ratatui::widgets::Block::default().style(Style::default().bg(Color::Rgb(11, 12, 21)));
        f.render_widget(bg_block, area);

        let term_rows = self.buf_h / 2;
        let term_cols = self.buf_w;
        let x_off = area.x + area.width.saturating_sub(term_cols as u16) / 2;
        let y_off = area.y + area.height.saturating_sub(term_rows as u16 + 4) / 2;

        let lines = render_logo_lines(&self.buffer, self.buf_w, self.buf_h, true, true);
        let render_area = Rect::new(x_off, y_off, term_cols as u16, (term_rows + 3) as u16);
        f.render_widget(Paragraph::new(lines), render_area);
    }
}

impl Default for SplashScreen {
    fn default() -> Self {
        Self::new()
    }
}

// ── Persistent logo window widget ───────────────────────────────────

/// Persistent logo rendered at reduced scale (40×30 chars).
/// Can be placed in a small workspace window (e.g. top-left corner).
pub fn build_logo_window_lines() -> Vec<Line<'static>> {
    let (buf, w, h) = logo_window_buffer();
    render_logo_lines(&buf, w, h, false, false)
}

pub fn logo_window_size() -> (u16, u16) {
    // 40 columns × 30 rows (60px / 2 half-block)
    (40, 30)
}
