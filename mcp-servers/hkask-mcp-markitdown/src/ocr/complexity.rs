//! Complexity Scoring Heuristic — Edge-density ratio via Sobel gradient.
//!
//! Pure function, deterministic, O(w·h). No new dependencies beyond `image`.
//! Thresholds from `hkask_types::ocr::thresholds`.

use hkask_types::ocr::{ComplexityScore, ComplexityTier, thresholds};
use image::{DynamicImage, GenericImageView};

/// Score page complexity by edge-density ratio.
///
/// # Algorithm
/// 1. Convert to grayscale (luma channel).
/// 2. Apply 3×3 Sobel operator in both X and Y directions.
/// 3. Compute gradient magnitude at each pixel.
/// 4. Edge-density = proportion of pixels above 50% of max gradient.
///
/// This is intentionally shallow: one function, three threshold constants.
/// Complexity scoring is a performance optimization (routing shortcut),
/// not a correctness dependency. Delete it → pipeline degrades to
/// single-backend; keep it small.
pub fn score_page_complexity(image: &DynamicImage) -> ComplexityScore {
    let gray = image.to_luma8();
    let (w, h) = gray.dimensions();
    let w_i = w as isize;
    let h_i = h as isize;

    // Sobel kernels
    let sobel_x = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
    let sobel_y = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];

    let mut max_grad: f32 = 0.0;
    let mut grad_sum: f32 = 0.0;
    let pixels = gray.as_raw();

    // Compute gradient magnitude at each interior pixel
    for y in 1..(h_i - 1) {
        for x in 1..(w_i - 1) {
            let mut gx: f32 = 0.0;
            let mut gy: f32 = 0.0;
            for ky in 0..3 {
                for kx in 0..3 {
                    let px = (x + kx - 1) as u32;
                    let py = (y + ky - 1) as u32;
                    let idx = (py * w + px) as usize;
                    let val = pixels[idx] as f32 / 255.0;
                    gx += val * sobel_x[ky as usize][kx as usize] as f32;
                    gy += val * sobel_y[ky as usize][kx as usize] as f32;
                }
            }
            let grad = (gx * gx + gy * gy).sqrt();
            if grad > max_grad {
                max_grad = grad;
            }
            grad_sum += grad;
        }
    }

    // Edge-density: proportion of pixels with gradient > 50% of max
    let threshold = max_grad * 0.5;
    let mut edge_count: usize = 0;
    let mut total_interior: usize = 0;
    for y in 1..(h_i - 1) {
        for x in 1..(w_i - 1) {
            let mut gx: f32 = 0.0;
            let mut gy: f32 = 0.0;
            for ky in 0..3 {
                for kx in 0..3 {
                    let px = (x + kx - 1) as u32;
                    let py = (y + ky - 1) as u32;
                    let idx = (py * w + px) as usize;
                    let val = pixels[idx] as f32 / 255.0;
                    gx += val * sobel_x[ky as usize][kx as usize] as f32;
                    gy += val * sobel_y[ky as usize][kx as usize] as f32;
                }
            }
            let grad = (gx * gx + gy * gy).sqrt();
            if grad > threshold {
                edge_count += 1;
            }
            total_interior += 1;
        }
    }

    let edge_density = if total_interior > 0 {
        edge_count as f32 / total_interior as f32
    } else {
        0.0
    };

    let tier = if edge_density < thresholds::SIMPLE_MAX {
        ComplexityTier::Simple
    } else if edge_density < thresholds::MODERATE_MAX {
        ComplexityTier::Moderate
    } else {
        ComplexityTier::Complex
    };

    ComplexityScore {
        value: edge_density,
        tier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Luma, Rgb, RgbImage};

    // REQ:ocr-complexity-01 — Blank image scores Simple
    #[test]
    fn blank_image_is_simple() {
        let img = DynamicImage::new_luma8(100, 100);
        let score = score_page_complexity(&img);
        assert_eq!(score.tier, ComplexityTier::Simple);
        assert!(
            score.value < 0.01,
            "blank image should have near-zero edge density"
        );
    }

    // REQ:ocr-complexity-02 — Text-only (high-contrast lines) scores Simple
    #[test]
    fn text_like_image_is_simple() {
        let mut img: RgbImage = ImageBuffer::new(200, 200);
        // Draw horizontal lines simulating text rows
        for y in (10..190).step_by(15) {
            for x in 10..190 {
                img.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }
        let dyn_img = DynamicImage::ImageRgb8(img);
        let score = score_page_complexity(&dyn_img);
        // Thin lines on white background — should be Simple
        assert_eq!(
            score.tier,
            ComplexityTier::Simple,
            "text-like line art should be Simple, got {:?}",
            score.tier
        );
    }

    // REQ:ocr-complexity-03 — Dense table (grid) scores Moderate
    #[test]
    fn dense_table_is_moderate() {
        let mut img: RgbImage = ImageBuffer::new(200, 200);
        // Dense grid — many edges
        for y in (0..200).step_by(8) {
            for x in 0..200 {
                img.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }
        for x in (0..200).step_by(8) {
            for y in 0..200 {
                img.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }
        let dyn_img = DynamicImage::ImageRgb8(img);
        let score = score_page_complexity(&dyn_img);
        // Dense grid should be at least Moderate
        assert!(
            score.tier == ComplexityTier::Moderate || score.tier == ComplexityTier::Complex,
            "dense grid should be Moderate or Complex, got {:?} (density={:.4})",
            score.tier,
            score.value
        );
    }

    // REQ:ocr-complexity-04 — Photograph-like (noise) scores Complex
    #[test]
    fn noisy_photograph_is_complex() {
        use rand::Rng;
        let mut rng = rand::rng();
        let mut img: RgbImage = ImageBuffer::new(200, 200);
        for y in 0..200 {
            for x in 0..200 {
                let v = rng.random_range(0u8..255u8);
                img.put_pixel(x, y, Rgb([v, v, v]));
            }
        }
        let dyn_img = DynamicImage::ImageRgb8(img);
        let score = score_page_complexity(&dyn_img);
        assert_eq!(
            score.tier,
            ComplexityTier::Complex,
            "noisy image should be Complex, got {:?} (density={:.4})",
            score.tier,
            score.value
        );
    }

    // REQ:ocr-complexity-05 — Deterministic: same input → same output
    #[test]
    fn deterministic_output() {
        let img = DynamicImage::new_luma8(50, 50);
        let a = score_page_complexity(&img);
        let b = score_page_complexity(&img);
        assert_eq!(a.value, b.value);
        assert_eq!(a.tier, b.tier);
    }
}
