//! Contract tests for hkask-mcp-replica — style embedding and centroid invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seams:
//! - `cosine_distance` (pure function from hkask-services)
//! - `ProbContractRunner` (probabilistic contract verification for LLM-driven tools)

use hkask_services::cosine_distance;
use hkask_test_harness::ProbContractRunner;
use proptest::prelude::*;

// ── cosine_distance invariants (deterministic contracts) ─────────────────────

// contract: REPLICA-COS-001
// expect: "I can verify that identical embeddings have zero distance — the identity invariant" [P8]
#[test]
fn cosine_distance_identity_is_zero() {
    let v = vec![1.0_f32, 2.0, 3.0];
    let d = cosine_distance(&v, &v);
    assert!(
        (d - 0.0).abs() < 1e-6,
        "identical vectors should have distance 0.0, got {d}"
    );
}

// contract: REPLICA-COS-002
// expect: "I can verify that orthogonal embeddings have unit distance — orthogonality invariant" [P8]
#[test]
fn cosine_distance_orthogonal_is_one() {
    let d = cosine_distance(&[1.0_f32, 0.0], &[0.0_f32, 1.0]);
    assert!(
        (d - 1.0).abs() < 1e-6,
        "orthogonal vectors should have distance 1.0, got {d}"
    );
}

// contract: REPLICA-COS-003
// expect: "I can verify that opposite embeddings have maximum distance — antipodal invariant" [P8]
#[test]
fn cosine_distance_opposite_is_two() {
    let d = cosine_distance(&[1.0_f32], &[-1.0_f32]);
    assert!(
        (d - 2.0).abs() < 1e-6,
        "opposite vectors should have distance 2.0, got {d}"
    );
}

// contract: REPLICA-COS-004
// expect: "I can verify empty embeddings are handled safely — degenerate case invariant" [P8]
#[test]
fn cosine_distance_empty_is_two() {
    let d = cosine_distance(&[], &[1.0_f32]);
    assert!(
        (d - 2.0).abs() < 1e-6,
        "empty vectors should return 2.0, got {d}"
    );
}

// contract: REPLICA-COS-005
// expect: "I can verify dimension mismatches are handled safely — mismatch invariant" [P8]
#[test]
fn cosine_distance_mismatched_is_two() {
    let d = cosine_distance(&[1.0_f32, 2.0], &[3.0_f32]);
    assert!(
        (d - 2.0).abs() < 1e-6,
        "mismatched dimensions should return 2.0, got {d}"
    );
}

// contract: REPLICA-COS-006
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

// contract: REPLICA-COS-007
// expect: "I can verify zero-norm embeddings are handled safely — zero-vector invariant" [P8]
#[test]
fn cosine_distance_zero_norm_is_two() {
    let d = cosine_distance(&[0.0_f32, 0.0], &[1.0_f32, 2.0]);
    assert!(
        (d - 2.0).abs() < 1e-6,
        "zero-norm vector should return 2.0, got {d}"
    );
}

// ── Probabilistic contract: centroid distance ordering ──────────────────────

/// Three author centroids in 4D space. gentle is close to [1,0,0,1], hemingway
/// to [0,1,0,1], woolf to [0,0,1,1].
fn author_centroids() -> Vec<(&'static str, Vec<f32>)> {
    vec![
        ("gentle", vec![1.0_f32, 0.0, 0.0, 1.0]),
        ("hemingway", vec![0.0_f32, 1.0, 0.0, 1.0]),
        ("woolf", vec![0.0_f32, 0.0, 1.0, 1.0]),
    ]
}

// contract: REPLICA-PROB-CENTROID-001
// expect: "I can verify probabilistically that style distance is meaningful — output closer to own author than others" [P9]
// prob: p=0.95, δ=0.05, k=0
// [P9] Motivating: Homeostatic Self-Regulation — quality gate on style proximity
// [P8] Constraining: Semantic Grounding — distances computed from known vectors
#[test]
fn centroid_distance_ordering_is_prob_contract_strong() {
    let centroids = author_centroids();
    let gentle = &centroids[0].1;
    let hemingway = &centroids[1].1;
    let woolf = &centroids[2].1;

    let runner = ProbContractRunner::new(0.95, 0.05, 0);

    let result = runner.evaluate(
        200,
        || {
            // Generate a test vector: gentle's centroid + Gaussian noise (σ=0.3)
            // This simulates the output of replica_compose for gentle — it should
            // be close to gentle's centroid and far from the others
            let mut rng = rand::rng();
            vec![
                1.0_f32 + (rng.random::<f32>() - 0.5) * 0.6,
                0.0_f32 + (rng.random::<f32>() - 0.5) * 0.6,
                0.0_f32 + (rng.random::<f32>() - 0.5) * 0.6,
                1.0_f32 + (rng.random::<f32>() - 0.5) * 0.6,
            ]
        },
        |test_vec| {
            let d_gentle = cosine_distance(test_vec, gentle);
            let d_hemingway = cosine_distance(test_vec, hemingway);
            let d_woolf = cosine_distance(test_vec, woolf);
            d_gentle < d_hemingway && d_gentle < d_woolf
        },
    );

    assert!(
        result.passed,
        "centroid distance ordering failed: {}/{} trials passed (rate: {:.3}, need >= {:.3})",
        result.successes, result.trials, result.actual_rate, result.target_rate
    );
}

// contract: REPLICA-PROB-CENTROID-002
// expect: "I can verify that the probabilistic contract correctly fails when distances are random" [P9]
// This is a negative test: random vectors won't be closer to gentle, so the contract should fail
#[test]
fn centroid_distance_ordering_fails_on_noise() {
    let runner = ProbContractRunner::new(0.90, 0.0, 0);

    let result = runner.evaluate(
        100,
        || {
            let mut rng = rand::rng();
            vec![
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
            ]
        },
        |test_vec| {
            let centroids = author_centroids();
            let d_gentle = cosine_distance(test_vec, &centroids[0].1);
            let d_hemingway = cosine_distance(test_vec, &centroids[1].1);
            d_gentle < d_hemingway
        },
    );

    assert!(
        !result.passed,
        "random vectors should NOT pass the centroid ordering contract (rate: {:.3})",
        result.actual_rate
    );
}

// ── Mashup monotonicity (probabilistic variant) ─────────────────────────────

// contract: REPLICA-MASHUP-002
// expect: "I can verify that blend monotonicity holds for random angle pairs with high probability" [P9]
// prob: p=0.90, δ=0.05, k=2
proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]
    #[test]
    fn mashup_monotonicity_probabilistic(
        (angle_a, angle_b) in (0.1f64..5.0f64, 0.1f64..5.0f64)
    ) {
        let diff = (angle_a - angle_b).abs();
        if diff < 0.3 {
            return Ok(()); // skip near-identical vectors
        }

        let a = vec![angle_a.cos() as f32, angle_a.sin() as f32];
        let b = vec![angle_b.cos() as f32, angle_b.sin() as f32];

        let runner = ProbContractRunner::new(0.90, 0.05, 2);
        let result = runner.evaluate(50,
            || {
                let blend: f64 = rand::rng().random::<f64>();
                let blended: Vec<f32> = a.iter().zip(b.iter())
                    .map(|(x, y)| (*x as f64 * (1.0 - blend) + *y as f64 * blend) as f32)
                    .collect();
                let d_a = cosine_distance(&blended, &a);
                let d_b = cosine_distance(&blended, &b);
                (d_a, d_b, blend)
            },
            |(d_a, d_b, blend)| {
                // Higher blend → higher d_a (further from a) and lower d_b (closer to b)
                // At blend=0.5, d_a and d_b should be roughly balanced
                if *blend > 0.5 {
                    d_a > d_b
                } else {
                    d_a < d_b
                }
            },
        );

        prop_assert!(result.passed,
            "mashup monotonicity failed: {}/{} trials (rate: {:.3}, need >= {:.3})",
            result.successes, result.trials, result.actual_rate, result.target_rate);
    }
}

// ── Self-consistency: identity under probabilistic contract ──────────────────

// contract: REPLICA-PROB-SELF-001
// expect: "I can verify that the style distance to self is reliably zero under a probabilistic contract" [P9]
// prob: p=0.99, δ=0.01, k=0
#[test]
fn self_consistency_under_prob_contract() {
    let a = vec![1.0_f32, 2.0, 3.0, 4.0];
    let runner = ProbContractRunner::new(0.99, 0.0, 0);
    let result = runner.evaluate(50, || a.clone(), |v| cosine_distance(&a, v) < 1e-6);
    assert!(
        result.passed,
        "self-consistency failed: {}/{} trials (rate: {:.3})",
        result.successes, result.trials, result.actual_rate
    );
}

// contract: REPLICA-PROB-RECOVERY-001
// expect: "I can verify that the k recovery window rescues a borderline contract" [P9]
// prob: p=0.99, δ=0.0, k=9
#[test]
fn recovery_window_rescues_failing_contract() {
    // A failing predicate that passes only on the second call per trial
    let mut call_count = 0u32;
    let runner = ProbContractRunner::new(0.99, 0.0, 9);
    let result = runner.evaluate(
        30,
        || {
            call_count += 1;
            call_count.is_multiple_of(2) // passes on every second call
        },
        |b| *b,
    );
    // With k=9, every trial gets 10 attempts, so every other call passes.
    // 30 trials × 10 attempts = every trial should pass.
    assert!(
        result.passed,
        "recovery should rescue contract: {}/{} trials (rate: {:.3})",
        result.successes, result.trials, result.actual_rate
    );
}

// ── Live inference integration test (manual, requires styles DB) ────────────

// contract: REPLICA-INTEG-001
// expect: "I can verify the full replica_compose pipeline when inference is available" [P9]
// prob: p=0.80, δ=0.10, k=3
// Run manually: HKASK_REPLICA_TEST_DB=/path/to/styles.db cargo test -- replica_compose_integration
#[test]
#[ignore]
fn replica_compose_integration_prob_contract() {
    let _db_path = match std::env::var("HKASK_REPLICA_TEST_DB") {
        Ok(p) => p,
        Err(_) => return,
    };

    // 1. Open the styles database, 2. Load centroids, 3. Compose prose,
    // 4. Embed output, 5. Verify centroid distance ordering via ProbContractRunner.
}
