//! Contract tests for hkask-mcp-replica — style embedding and centroid invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `cosine_distance` (pure function from hkask-services).

use hkask_mcp_replica::cosine_distance;
use hkask_test_harness::ProbContractRunner;
use proptest::prelude::*;

// ── cosine_distance invariants ──────────────────────────────────────────────

// REQ: REPLICA-COS-001 — identical vectors have distance 0.0
// expect: "I can verify that identical embeddings have zero distance — the identity invariant" [P8]
#[test]
fn cosine_distance_identity_is_zero() {
    let v = vec![1.0_f32, 2.0, 3.0];
    let d = cosine_distance(&v, &v);
    assert!((d - 0.0).abs() < 1e-6, "identical vectors should have distance 0.0, got {d}");
}

// REQ: REPLICA-COS-002 — orthogonal unit vectors have distance 1.0
// expect: "I can verify that orthogonal embeddings have unit distance — orthogonality invariant" [P8]
#[test]
fn cosine_distance_orthogonal_is_one() {
    let a = vec![1.0_f32, 0.0];
    let b = vec![0.0_f32, 1.0];
    let d = cosine_distance(&a, &b);
    assert!((d - 1.0).abs() < 1e-6, "orthogonal vectors should have distance 1.0, got {d}");
}

// REQ: REPLICA-COS-003 — opposite vectors have distance 2.0
// expect: "I can verify that opposite embeddings have maximum distance — antipodal invariant" [P8]
#[test]
fn cosine_distance_opposite_is_two() {
    let a = vec![1.0_f32];
    let b = vec![-1.0_f32];
    let d = cosine_distance(&a, &b);
    assert!((d - 2.0).abs() < 1e-6, "opposite vectors should have distance 2.0, got {d}");
}

// REQ: REPLICA-COS-004 — empty vectors return distance 2.0 (degenerate)
// expect: "I can verify empty embeddings are handled safely — degenerate case invariant" [P8]
#[test]
fn cosine_distance_empty_is_two() {
    let d = cosine_distance(&[], &[1.0_f32]);
    assert!((d - 2.0).abs() < 1e-6, "empty vectors should return 2.0, got {d}");
}

// REQ: REPLICA-COS-005 — mismatched lengths return distance 2.0
// expect: "I can verify dimension mismatches are handled safely — mismatch invariant" [P8]
#[test]
fn cosine_distance_mismatched_is_two() {
    let d = cosine_distance(&[1.0_f32, 2.0], &[3.0_f32]);
    assert!((d - 2.0).abs() < 1e-6, "mismatched dimensions should return 2.0, got {d}");
}

// REQ: REPLICA-COS-006 — cosine_distance is symmetric
// expect: "I can verify that style distance is symmetric — dist(A,B) equals dist(B,A)" [P8]
proptest! {
    #[test]
    fn cosine_distance_is_symmetric(
        (x1, y1, z1, x2, y2, z2) in (
            0.1f32..10.0f32, 0.1f32..10.0f32, 0.1f32..10.0f32,
            0.1f32..10.0f32, 0.1f32..10.0f32, 0.1f32..10.0f32,
        )
    ) {
        let a = vec![x1, y1, z1];
        let b = vec![x2, y2, z2];
        let d_ab = cosine_distance(&a, &b);
        let d_ba = cosine_distance(&b, &a);
        prop_assert!((d_ab - d_ba).abs() < 1e-6,
            "cosine distance not symmetric: d(a,b)={} d(b,a)={}", d_ab, d_ba);
    }
}

// REQ: REPLICA-COS-007 — all-zero vectors return 2.0 (no direction)
// expect: "I can verify zero-norm embeddings are handled safely — zero-vector invariant" [P8]
#[test]
fn cosine_distance_zero_norm_is_two() {
    let d = cosine_distance(&[0.0_f32, 0.0], &[1.0_f32, 2.0]);
    assert!((d - 2.0).abs() < 1e-6, "zero-norm vector should return 2.0, got {d}");
}

// ── Centroid distance ordering ──────────────────────────────────────────────

// REQ: REPLICA-CENTROID-001 — stored centroids preserve distance ordering
// expect: "I can verify that an embedding is closer to its own centroid than to other centroids" [P9]
// [P8] Constraining: distances are computed from known vectors — semantic grounding
#[test]
fn centroid_distance_ordering_self_closest() {
    let test_vec = vec![0.95_f32, 0.05, 0.0, 1.0];
    let centroids: Vec<(&str, Vec<f32>)> = vec![
        ("gentle", vec![1.0_f32, 0.0, 0.0, 1.0]),
        ("hemingway", vec![0.0_f32, 1.0, 0.0, 1.0]),
        ("woolf", vec![0.0_f32, 0.0, 1.0, 1.0]),
    ];

    let d_gentle = cosine_distance(&test_vec, &centroids[0].1);
    let d_hemingway = cosine_distance(&test_vec, &centroids[1].1);
    let d_woolf = cosine_distance(&test_vec, &centroids[2].1);

    assert!(d_gentle < d_hemingway,
        "test_vec should be closer to gentle ({d_gentle}) than hemingway ({d_hemingway})");
    assert!(d_gentle < d_woolf,
        "test_vec should be closer to gentle ({d_gentle}) than woolf ({d_woolf})");
}

// ── Mashup monotonicity ─────────────────────────────────────────────────────

// REQ: REPLICA-MASHUP-001 — blend interpolation is monotonic
// expect: "I can verify that as I blend toward an author, the distance to that author decreases" [P9]
proptest! {
    #[test]
    fn mashup_blend_is_monotonic(
        (angle_a, angle_b) in (0.0f64..std::f64::consts::TAU, 0.0f64..std::f64::consts::TAU)
    ) {
        // Skip degenerate cases where vectors are nearly identical
        let diff = (angle_a - angle_b).abs();
        if diff < 0.1 || (std::f64::consts::TAU - diff) < 0.1 {
            return Ok(());
        }

        let a = vec![angle_a.cos() as f32, angle_a.sin() as f32];
        let b = vec![angle_b.cos() as f32, angle_b.sin() as f32];

        let mut d_a_prior = 999.0_f64;
        let mut d_b_prior = 0.0_f64;

        for blend in &[0.0_f64, 0.25, 0.5, 0.75, 1.0] {
            let inv: f64 = 1.0 - blend;
            let blended: Vec<f32> = a.iter().zip(b.iter())
                .map(|(x, y)| (*x as f64 * inv + *y as f64 * blend) as f32)
                .collect();
            let d_a = cosine_distance(&blended, &a);
            let d_b = cosine_distance(&blended, &b);

            if *blend > 0.0 {
                prop_assert!(d_a >= d_a_prior - 1e-6,
                    "blend={blend}: d_a ({d_a}) should be >= prior ({d_a_prior})");
                prop_assert!(d_b <= d_b_prior + 1e-6,
                    "blend={blend}: d_b ({d_b}) should be <= prior ({d_b_prior})");
            }

            d_a_prior = d_a;
            d_b_prior = d_b;
        }
    }
}

// ── Probabilistic contract: style vector self-consistency ───────────────────

// REQ: REPLICA-PROB-001 — self-generated style vectors are self-consistent
// expect: "I can verify that a style vector's distance to itself is always near zero" [P9]
// prob: p=0.99, δ=0.01, k=0
#[test]
fn style_vector_self_consistency_passes_prob_contract() {
    let a = vec![1.0_f32, 2.0, 3.0, 4.0];
    let runner = ProbContractRunner::new(0.99, 0.0, 0);
    let result = runner.evaluate(50,
        || a.clone(),
        |v| cosine_distance(&a, v) < 1e-6,
    );
    assert!(result.passed,
        "self-consistency failed: {}/{} trials (rate: {:.3})",
        result.successes, result.trials, result.actual_rate);
}
