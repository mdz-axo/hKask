//! Splash screen — faithful terminal reproduction of the Kask SVG logo.
//!
//! Rasterizes the SVG geometry (`assets/kask-logo.svg`) into a pixel buffer
//! and renders it using Unicode half-block characters (`▀ ▄ █`). The four
//! compositional elements are preserved:
//!
//! 1. **Galvanized milk jug** — tapered body, neck, rim, pouring spout, bail handle, ribbing
//! 2. **Calligraphic stroke variation** — thick (2px) downstrokes, thin (1px) transitions
//! 3. **Curator's Eye** — almond eyelids, filled iris, pupil, white reflection, operator gap
//! 4. **Bitemporal shadow** — same geometry offset left at reduced opacity
//!
//! The pixel buffer is computed once from the SVG coordinates and cached.

use crossterm::event::{self, Event};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use std::time::{Duration, Instant};

/// Duration to display the splash screen before auto-transitioning.
const SPLASH_DURATION_MS: u64 = 2500;

// ── Pixel buffer constants ──────────────────────────────────────────
//
// The SVG viewBox is 400×600. We rasterize at scale 0.2 → 80×120 pixels.
// Each terminal row = 2 pixel rows (half-block rendering), so 60 terminal rows.
//
// Pixel values:
//   0 = background (transparent)
//   1 = main stroke (#1A1A1A → light gray on dark bg)
//   2 = shadow stroke (15% opacity → dim gray)
//   3 = highlight (white — eye reflection, operator gap)

const CANVAS_W: usize = 80;
const CANVAS_H: usize = 120;
const SCALE: f64 = 0.2;

/// Pre-computed pixel buffer for the Kask logo.
/// Computed once via `build_logo_buffer()`.
fn build_logo_buffer() -> Vec<u8> {
    let mut buf = vec![0u8; CANVAS_W * CANVAS_H];

    // ── Helper: scale SVG coordinate to canvas coordinate ──
    let sx = |x: f64| -> f64 { x * SCALE };
    let sy = |y: f64| -> f64 { y * SCALE };

    // ── BITEMPORAL SHADOW (drawn first, so main strokes overlay) ──
    // Shadow body — tapered jug: M 160 180 L 140 440 Q 140 460 200 460 Q 260 460 260 440 L 240 180 Z
    // Offset left by 25 SVG units → translate(-25, 0)
    let so = -25.0; // shadow offset
    // Shadow left side
    draw_thick_line(
        &mut buf,
        sx(160.0 + so),
        sy(180.0),
        sx(140.0 + so),
        sy(440.0),
        1,
        2,
    );
    // Shadow bottom curve (approximate with lines)
    draw_thick_line(
        &mut buf,
        sx(140.0 + so),
        sy(440.0),
        sx(140.0 + so),
        sy(460.0),
        1,
        2,
    );
    draw_thick_line(
        &mut buf,
        sx(140.0 + so),
        sy(460.0),
        sx(200.0 + so),
        sy(460.0),
        1,
        2,
    );
    draw_thick_line(
        &mut buf,
        sx(200.0 + so),
        sy(460.0),
        sx(260.0 + so),
        sy(460.0),
        1,
        2,
    );
    draw_thick_line(
        &mut buf,
        sx(260.0 + so),
        sy(460.0),
        sx(260.0 + so),
        sy(440.0),
        1,
        2,
    );
    // Shadow right side
    draw_thick_line(
        &mut buf,
        sx(260.0 + so),
        sy(440.0),
        sx(240.0 + so),
        sy(180.0),
        1,
        2,
    );
    // Shadow top
    draw_thick_line(
        &mut buf,
        sx(240.0 + so),
        sy(180.0),
        sx(160.0 + so),
        sy(180.0),
        1,
        2,
    );
    // Shadow neck rect: x=170 y=145 w=60 h=40
    draw_rect_stroke(
        &mut buf,
        sx(170.0 + so),
        sy(145.0),
        sx(60.0),
        sy(40.0),
        1,
        2,
    );
    // Shadow bail handle: M 165 160 C 165 80 200 60 200 60 C 200 60 235 80 235 160
    draw_cubic_bezier(
        &mut buf,
        sx(165.0 + so),
        sy(160.0),
        sx(165.0 + so),
        sy(80.0),
        sx(200.0 + so),
        sy(60.0),
        sx(200.0 + so),
        sy(60.0),
        1,
        2,
    );
    draw_cubic_bezier(
        &mut buf,
        sx(200.0 + so),
        sy(60.0),
        sx(200.0 + so),
        sy(60.0),
        sx(235.0 + so),
        sy(80.0),
        sx(235.0 + so),
        sy(160.0),
        1,
        2,
    );

    // ── MAIN JUG BODY ──

    // Left side — thick downstroke, tapered: M 160 180 L 140 440, width 9
    draw_thick_line(&mut buf, sx(160.0), sy(180.0), sx(140.0), sy(440.0), 2, 1);
    // Right side — thick downstroke, tapered: M 240 180 L 260 440, width 9
    draw_thick_line(&mut buf, sx(240.0), sy(180.0), sx(260.0), sy(440.0), 2, 1);
    // Bottom — thick curved base: M 140 440 Q 140 460 200 460 Q 260 460 260 440, width 9
    draw_quad_bezier(
        &mut buf,
        sx(140.0),
        sy(440.0),
        sx(140.0),
        sy(460.0),
        sx(200.0),
        sy(460.0),
        2,
        1,
    );
    draw_quad_bezier(
        &mut buf,
        sx(200.0),
        sy(460.0),
        sx(260.0),
        sy(460.0),
        sx(260.0),
        sy(440.0),
        2,
        1,
    );
    // Top shoulder — thinner transition: M 160 180 Q 200 175 240 180, width 7
    draw_quad_bezier(
        &mut buf,
        sx(160.0),
        sy(180.0),
        sx(200.0),
        sy(175.0),
        sx(240.0),
        sy(180.0),
        1,
        1,
    );

    // ── GALVANIZED RIBBING — three horizontal reinforcement bands ──
    // Upper rib: M 155 240 Q 200 238 245 240, width 5
    draw_quad_bezier(
        &mut buf,
        sx(155.0),
        sy(240.0),
        sx(200.0),
        sy(238.0),
        sx(245.0),
        sy(240.0),
        1,
        1,
    );
    // Middle rib: M 150 320 Q 200 318 250 320, width 5
    draw_quad_bezier(
        &mut buf,
        sx(150.0),
        sy(320.0),
        sx(200.0),
        sy(318.0),
        sx(250.0),
        sy(320.0),
        1,
        1,
    );
    // Lower rib: M 145 400 Q 200 398 255 400, width 5
    draw_quad_bezier(
        &mut buf,
        sx(145.0),
        sy(400.0),
        sx(200.0),
        sy(398.0),
        sx(255.0),
        sy(400.0),
        1,
        1,
    );

    // ── NECK ──
    // Left neck: M 170 145 L 170 185, width 8
    draw_thick_line(&mut buf, sx(170.0), sy(145.0), sx(170.0), sy(185.0), 2, 1);
    // Right neck: M 230 145 L 230 185, width 8
    draw_thick_line(&mut buf, sx(230.0), sy(145.0), sx(230.0), sy(185.0), 2, 1);
    // Neck top: M 170 145 Q 200 140 230 145, width 7
    draw_quad_bezier(
        &mut buf,
        sx(170.0),
        sy(145.0),
        sx(200.0),
        sy(140.0),
        sx(230.0),
        sy(145.0),
        1,
        1,
    );

    // ── RIM — rolled edge at top ──
    // M 165 145 C 165 138 200 135 235 145 C 235 152 200 155 165 145, width 6
    draw_cubic_bezier(
        &mut buf,
        sx(165.0),
        sy(145.0),
        sx(165.0),
        sy(138.0),
        sx(200.0),
        sy(135.0),
        sx(235.0),
        sy(145.0),
        1,
        1,
    );
    draw_cubic_bezier(
        &mut buf,
        sx(235.0),
        sy(145.0),
        sx(235.0),
        sy(152.0),
        sx(200.0),
        sy(155.0),
        sx(165.0),
        sy(145.0),
        1,
        1,
    );

    // ── POURING SPOUT — small lip on left side of rim ──
    // M 165 145 C 155 140 150 135 145 130, width 7
    draw_cubic_bezier(
        &mut buf,
        sx(165.0),
        sy(145.0),
        sx(155.0),
        sy(140.0),
        sx(150.0),
        sy(135.0),
        sx(145.0),
        sy(130.0),
        1,
        1,
    );

    // ── BAIL HANDLE — wire arch from ear to ear ──
    // Left ear: cx=165 cy=160 r=4 (filled circle)
    draw_filled_circle(&mut buf, sx(165.0), sy(160.0), sx(4.0), 1);
    // Right ear: cx=235 cy=160 r=4 (filled circle)
    draw_filled_circle(&mut buf, sx(235.0), sy(160.0), sx(4.0), 1);
    // Bail wire: M 165 160 C 165 80 200 60 200 60 C 200 60 235 80 235 160, width 6
    draw_cubic_bezier(
        &mut buf,
        sx(165.0),
        sy(160.0),
        sx(165.0),
        sy(80.0),
        sx(200.0),
        sy(60.0),
        sx(200.0),
        sy(60.0),
        1,
        1,
    );
    draw_cubic_bezier(
        &mut buf,
        sx(200.0),
        sy(60.0),
        sx(200.0),
        sy(60.0),
        sx(235.0),
        sy(80.0),
        sx(235.0),
        sy(160.0),
        1,
        1,
    );
    // Bail grip — thicker section at top: M 190 65 Q 200 60 210 65, width 9
    draw_quad_bezier(
        &mut buf,
        sx(190.0),
        sy(65.0),
        sx(200.0),
        sy(60.0),
        sx(210.0),
        sy(65.0),
        2,
        1,
    );

    // ── CURATOR'S EYE ──
    // Upper eyelid — thick: M 175 310 Q 200 295 225 310, width 8
    draw_quad_bezier(
        &mut buf,
        sx(175.0),
        sy(310.0),
        sx(200.0),
        sy(295.0),
        sx(225.0),
        sy(310.0),
        2,
        1,
    );
    // Lower eyelid — thinner: M 178 330 Q 200 350 222 330, width 5
    draw_quad_bezier(
        &mut buf,
        sx(178.0),
        sy(330.0),
        sx(200.0),
        sy(350.0),
        sx(222.0),
        sy(330.0),
        1,
        1,
    );

    // Iris — solid fill: cx=200 cy=320 r=18
    draw_filled_circle(&mut buf, sx(200.0), sy(320.0), sx(18.0), 1);
    // Pupil — darker: cx=200 cy=320 r=10
    draw_filled_circle(&mut buf, sx(200.0), sy(320.0), sx(10.0), 1);
    // Light reflection — white: cx=205 cy=315 r=5
    draw_filled_circle(&mut buf, sx(205.0), sy(315.0), sx(5.0), 3);

    // Eyelashes — subtle flicks (width 2, very short)
    draw_thick_line(&mut buf, sx(188.0), sy(308.0), sx(185.0), sy(302.0), 1, 1);
    draw_thick_line(&mut buf, sx(196.0), sy(305.0), sx(194.0), sy(298.0), 1, 1);
    draw_thick_line(&mut buf, sx(204.0), sy(305.0), sx(206.0), sy(298.0), 1, 1);
    draw_thick_line(&mut buf, sx(212.0), sy(308.0), sx(215.0), sy(302.0), 1, 1);

    // Operator Authority Gap — white arc: M 208 312 A 18 18 0 0 1 210 328, width 3
    // Approximate as a short line segment (the arc is very small at this scale)
    draw_thick_line(&mut buf, sx(208.0), sy(312.0), sx(210.0), sy(328.0), 1, 3);

    buf
}

// ── Rasterization primitives ────────────────────────────────────────

fn set_pixel(buf: &mut [u8], x: f64, y: f64, value: u8) {
    let ix = x.round() as i32;
    let iy = y.round() as i32;
    if ix >= 0 && (ix as usize) < CANVAS_W && iy >= 0 && (iy as usize) < CANVAS_H {
        let idx = iy as usize * CANVAS_W + ix as usize;
        // Main stroke (1) and highlight (3) override shadow (2); shadow doesn't override main
        if value == 1 || value == 3 || buf[idx] == 0 {
            buf[idx] = value;
        }
    }
}

/// Bresenham line with thickness (draws parallel offset lines for width > 1).
fn draw_thick_line(buf: &mut [u8], x0: f64, y0: f64, x1: f64, y1: f64, width: i32, value: u8) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        set_pixel(buf, x0, y0, value);
        return;
    }
    // Normal vector perpendicular to line
    let nx = -dy / len;
    let ny = dx / len;
    let half_w = (width as f64) / 2.0;

    for w in 0..width {
        let offset = (w as f64) - half_w + 0.5;
        let ox = nx * offset;
        let oy = ny * offset;
        draw_line_bresenham(buf, x0 + ox, y0 + oy, x1 + ox, y1 + oy, value);
    }
}

fn draw_line_bresenham(buf: &mut [u8], x0: f64, y0: f64, x1: f64, y1: f64, value: u8) {
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
        if x >= 0 && (x as usize) < CANVAS_W && y >= 0 && (y as usize) < CANVAS_H {
            let idx = y as usize * CANVAS_W + x as usize;
            if value == 1 || value == 3 || buf[idx] == 0 {
                buf[idx] = value;
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

/// Quadratic Bezier — subdivide into line segments.
fn draw_quad_bezier(
    buf: &mut [u8],
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
    let mut prev_x = x0;
    let mut prev_y = y0;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let mt = 1.0 - t;
        let px = mt * mt * x0 + 2.0 * mt * t * x1 + t * t * x2;
        let py = mt * mt * y0 + 2.0 * mt * t * y1 + t * t * y2;
        draw_thick_line(buf, prev_x, prev_y, px, py, width, value);
        prev_x = px;
        prev_y = py;
    }
}

/// Cubic Bezier — subdivide into line segments.
fn draw_cubic_bezier(
    buf: &mut [u8],
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
    let mut prev_x = x0;
    let mut prev_y = y0;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let mt = 1.0 - t;
        let px =
            mt * mt * mt * x0 + 3.0 * mt * mt * t * x1 + 3.0 * mt * t * t * x2 + t * t * t * x3;
        let py =
            mt * mt * mt * y0 + 3.0 * mt * mt * t * y1 + 3.0 * mt * t * t * y2 + t * t * t * y3;
        draw_thick_line(buf, prev_x, prev_y, px, py, width, value);
        prev_x = px;
        prev_y = py;
    }
}

/// Filled circle using midpoint algorithm.
fn draw_filled_circle(buf: &mut [u8], cx: f64, cy: f64, r: f64, value: u8) {
    let r_int = r.round() as i32;
    let cx_int = cx.round() as i32;
    let cy_int = cy.round() as i32;
    for dy in -r_int..=r_int {
        for dx in -r_int..=r_int {
            if dx * dx + dy * dy <= r_int * r_int {
                let px = cx_int + dx;
                let py = cy_int + dy;
                if px >= 0 && (px as usize) < CANVAS_W && py >= 0 && (py as usize) < CANVAS_H {
                    let idx = py as usize * CANVAS_W + px as usize;
                    if value == 1 || value == 3 || buf[idx] == 0 {
                        buf[idx] = value;
                    }
                }
            }
        }
    }
}

/// Stroked rectangle (outline only).
fn draw_rect_stroke(buf: &mut [u8], x: f64, y: f64, w: f64, h: f64, width: i32, value: u8) {
    // Top
    draw_thick_line(buf, x, y, x + w, y, width, value);
    // Bottom
    draw_thick_line(buf, x, y + h, x + w, y + h, width, value);
    // Left
    draw_thick_line(buf, x, y, x, y + h, width, value);
    // Right
    draw_thick_line(buf, x + w, y, x + w, y + h, width, value);
}

// ── Splash screen widget ────────────────────────────────────────────

/// Kask logo splash screen — faithful reproduction of `kask-logo.svg`.
pub struct SplashScreen {
    start_time: Instant,
    duration: Duration,
    buffer: Vec<u8>,
}

impl SplashScreen {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            duration: Duration::from_millis(SPLASH_DURATION_MS),
            buffer: build_logo_buffer(),
        }
    }

    /// Check if the splash screen should auto-dismiss.
    pub fn should_dismiss(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }

    /// Check for early dismissal via key press.
    pub fn check_early_dismiss(&mut self) -> bool {
        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            if let Ok(Event::Key(_)) = event::read() {
                return true;
            }
        }
        false
    }

    /// Render the splash screen into the given frame.
    pub fn render(&self, f: &mut Frame) {
        let area = f.area();

        // Clear to dark background
        let bg = Block::default().style(Style::default().bg(Color::Rgb(11, 12, 21)));
        f.render_widget(bg, area);

        // The pixel buffer is CANVAS_W × CANVAS_H.
        // With half-block rendering, terminal rows = CANVAS_H / 2.
        let term_rows = CANVAS_H / 2;
        let term_cols = CANVAS_W;

        // Center the logo in the terminal
        let x_off = area.x + area.width.saturating_sub(term_cols as u16) / 2;
        let y_off = area.y + area.height.saturating_sub(term_rows as u16 + 4) / 2;

        // Render pixel buffer using half-block characters
        let mut lines: Vec<Line> = Vec::new();

        for row in 0..term_rows {
            let mut spans: Vec<Span> = Vec::new();
            let top_y = row * 2;
            let bot_y = row * 2 + 1;

            for col in 0..term_cols {
                let top = self.buffer[top_y * CANVAS_W + col];
                let bot = self.buffer[bot_y * CANVAS_W + col];

                let (ch, fg, bg_color) = half_block_pixel(top, bot);
                spans.push(Span::styled(ch, Style::default().fg(fg).bg(bg_color)));
            }
            lines.push(Line::from(spans));
        }

        // Add "KASK" wordmark below the logo (monospace, letter-spaced)
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{:>width$}", "K  A  S  K", width = term_cols / 2 + 8),
            Style::default()
                .fg(Color::Rgb(224, 224, 224))
                .bg(Color::Rgb(11, 12, 21)),
        )));

        // Prompt
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

        let render_area = Rect::new(x_off, y_off, term_cols as u16, (term_rows + 3) as u16);
        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, render_area);
    }
}

/// Map a pair of vertical pixels (top, bottom) to a half-block character + colors.
///
/// Pixel values: 0=bg, 1=main stroke, 2=shadow, 3=highlight
fn half_block_pixel(top: u8, bot: u8) -> (&'static str, Color, Color) {
    let bg = Color::Rgb(11, 12, 21); // #0B0C15 deep space
    let main = Color::Rgb(224, 224, 224); // #E0E0E0 light gray
    let shadow = Color::Rgb(60, 60, 70); // dim shadow
    let highlight = Color::White; // eye reflection

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

impl Default for SplashScreen {
    fn default() -> Self {
        Self::new()
    }
}
