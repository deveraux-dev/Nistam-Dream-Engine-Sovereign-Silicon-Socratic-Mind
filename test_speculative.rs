use std::fmt::Debug;

pub trait Token: Copy + Eq + Debug {}
impl<T: Copy + Eq + Debug> Token for T {}

pub struct Verification<T: Token> {
    pub accepted: Vec<T>,
    pub correction: T,
}

pub trait DraftModel<T: Token> {
    fn draft(&mut self, context: &[T], gamma: usize) -> Vec<T>;
}

pub trait TargetModel<T: Token> {
    fn verify(&mut self, context: &[T], candidates: &[T]) -> Verification<T>;
}

pub struct SpeculativeEngine<D, V, T: Token> {
    draft: D,
    target: V,
    pub stats: SpeculativeStats,
    _marker: std::marker::PhantomData<T>,
}

#[derive(Debug, Default)]
pub struct SpeculativeStats {
    pub total_generated: usize,
    pub total_drafted: usize,
    pub total_accepted: usize,
    pub iterations: usize,
}

impl<D, V, T> SpeculativeEngine<D, V, T>
where
    T: Token,
    D: DraftModel<T>,
    V: TargetModel<T>,
{
    pub fn new(draft: D, target: V) -> Self {
        Self {
            draft,
            target,
            stats: SpeculativeStats::default(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn generate(&mut self, prompt: &[T], gamma: usize, max_length: usize) -> Vec<T> {
        let mut context = prompt.to_vec();

        while context.len() < max_length {
            self.stats.iterations += 1;

            let remaining = max_length - context.len();
            let lookahead = gamma.min(remaining);
            let candidates = self.draft.draft(&context, lookahead);

            if candidates.is_empty() {
                break;
            }

            self.stats.total_drafted += candidates.len();

            let verification = self.target.verify(&context, &candidates);

            let accepted_count = verification.accepted.len();
            self.stats.total_accepted += accepted_count;
            self.stats.total_generated += accepted_count + 1;

            context.extend(verification.accepted);
            context.push(verification.correction);
        }

        context
    }
}

// Test Case 1: All-Accept Path (ISSUE A)
struct AllAcceptDraft;
impl DraftModel<u32> for AllAcceptDraft {
    fn draft(&mut self, context: &[u32], gamma: usize) -> Vec<u32> {
        let last = *context.last().unwrap_or(&0);
        (1..=gamma as u32).map(|i| last + i).collect()
    }
}

struct AllAcceptTarget;
impl TargetModel<u32> for AllAcceptTarget {
    fn verify(&mut self, context: &[u32], candidates: &[u32]) -> Verification<u32> {
        let last = *context.last().unwrap_or(&0);
        Verification {
            accepted: candidates.to_vec(),
            correction: last + candidates.len() as u32 + 1,
        }
    }
}

// Test Case 2: Partial Accept (ISSUE B)
struct PartialDraft;
impl DraftModel<u32> for PartialDraft {
    fn draft(&mut self, context: &[u32], gamma: usize) -> Vec<u32> {
        let last = *context.last().unwrap_or(&0);
        (1..=gamma as u32).map(|i| last + i).collect()
    }
}

struct PartialTarget;
impl TargetModel<u32> for PartialTarget {
    fn verify(&mut self, context: &[u32], candidates: &[u32]) -> Verification<u32> {
        let last = *context.last().unwrap_or(&0);
        let mut accepted = Vec::new();

        for (i, &cand) in candidates.iter().enumerate() {
            let expected = last + (i as u32) + 1;
            if cand == expected && cand % 2 == 0 {
                accepted.push(cand);
            } else {
                return Verification {
                    accepted,
                    correction: expected + 100,
                };
            }
        }

        Verification {
            accepted,
            correction: last + candidates.len() as u32 + 1,
        }
    }
}

// Test Case 3: Zero Draft (ISSUE C)
struct ZeroDraft;
impl DraftModel<u32> for ZeroDraft {
    fn draft(&mut self, _context: &[u32], _gamma: usize) -> Vec<u32> {
        vec![] // Always returns empty
    }
}

struct ZeroTarget;
impl TargetModel<u32> for ZeroTarget {
    fn verify(&mut self, _context: &[u32], _candidates: &[u32]) -> Verification<u32> {
        Verification {
            accepted: vec![],
            correction: 999,
        }
    }
}

// Test Case 4: Divergence Early (ISSUE D)
struct DivergeEarlyDraft {
    iter: usize,
}

impl DraftModel<u32> for DivergeEarlyDraft {
    fn draft(&mut self, context: &[u32], gamma: usize) -> Vec<u32> {
        let last = *context.last().unwrap_or(&0);
        self.iter += 1;

        if self.iter == 1 {
            // First call: return 4 candidates
            (1..=gamma as u32).map(|i| last + i).collect()
        } else {
            // Second call: return 2 candidates
            vec![last + 1, last + 2]
        }
    }
}

struct DivergeEarlyTarget;
impl TargetModel<u32> for DivergeEarlyTarget {
    fn verify(&mut self, context: &[u32], candidates: &[u32]) -> Verification<u32> {
        let last = *context.last().unwrap_or(&0);

        // Reject everything in first iteration
        Verification {
            accepted: vec![],
            correction: 5000,
        }
    }
}

fn main() {
    println!("=== SPECULATIVE DECODING ENGINE TESTS ===\n");

    // Test 1: All-Accept Path
    println!("TEST 1: All-Accept Path (POTENTIAL ISSUE A)");
    let mut engine1 = SpeculativeEngine::new(AllAcceptDraft, AllAcceptTarget);
    let prompt = vec![0u32];
    let tokens1 = engine1.generate(&prompt, 4, 10);
    println!("Result: {:?}", tokens1);
    println!("Stats: {:?}", engine1.stats);
    println!("Expected: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]");
    println!("ISSUE: Appending correction after all-accept means we get: [0, 1,2,3,4, CORRECTION_5, ...]");
    println!("       This is correct, but verify it's intended behavior.\n");

    // Test 2: Partial Accept Leading to Divergence
    println!("TEST 2: Partial Accept with Mid-Sequence Rejection (ISSUE B)");
    let mut engine2 = SpeculativeEngine::new(PartialDraft, PartialTarget);
    let prompt = vec![0u32];
    let tokens2 = engine2.generate(&prompt, 4, 12);
    println!("Result: {:?}", tokens2);
    println!("Stats: {:?}", engine2.stats);
    println!("Expected sequence: [0, 2 (first even), 4 (second even), ...]");
    println!("ISSUE: If draft predicts [1,2,3,4] and target rejects on first odd (1),");
    println!("       verification returns correction=101 instead of ground truth.\n");

    // Test 3: Empty Draft (Stall Condition)
    println!("TEST 3: Empty Draft Returns (ISSUE C - HARD LOOP STALL)");
    let mut engine3 = SpeculativeEngine::new(ZeroDraft, ZeroTarget);
    let prompt = vec![0u32];
    println!("Running generate with max_length=10...");
    let start = std::time::Instant::now();
    let tokens3 = engine3.generate(&prompt, 4, 10);
    let elapsed = start.elapsed();
    println!("Result: {:?}", tokens3);
    println!("Stats: {:?}", engine3.stats);
    println!("Time: {:?}", elapsed);
    println!("CRITICAL ISSUE: Draft returns 0 candidates, context never grows.");
    println!("               Loop breaks immediately, target length never reached.\n");

    // Test 4: Late Divergence Pattern
    println!("TEST 4: Gamma Variance (Lookahead Shrinking)");
    let mut engine4 = SpeculativeEngine::new(DivergeEarlyDraft { iter: 0 }, DivergeEarlyTarget);
    let prompt = vec![0u32];
    let tokens4 = engine4.generate(&prompt, 4, 15);
    println!("Result: {:?}", tokens4);
    println!("Stats: {:?}", engine4.stats);
    println!("ISSUE: Verification accepts 0 tokens, appends correction=5000.");
    println!("       Next iteration: context=[0, 5000], gamma=lookahead=min(4, 14)");
    println!("       Draft sees context.last()=5000, predicts 5001..5004");
    println!("       Completely diverged from intended sequence.\n");

    // Test 5: Boundary Condition (Gamma Clamping)
    println!("TEST 5: Gamma Clamping on Final Iteration");
    let mut engine5 = SpeculativeEngine::new(AllAcceptDraft, AllAcceptTarget);
    let prompt = vec![0u32];
    let tokens5 = engine5.generate(&prompt, 4, 7);
    println!("Result: {:?}", tokens5);
    println!("Stats: {:?}", engine5.stats);
    println!("Trace: Start context=[0], max_length=7, gamma=4");
    println!("       Iter 1: draft 4 tokens, accept all, + correction → context=[0,1,2,3,4,5]");
    println!("       Iter 2: remaining=1, gamma=min(4,1)=1, draft returns [6]");
    println!("               verify accepts [6], adds correction=7 → context=[0..7]");
    println!("       Should stop at exactly 7.\n");

    println!("\n=== SUMMARY OF ISSUES ===");
    println!("A. Correction Overload: Every iteration appends 1 correction, even if all draft tokens accepted.");
    println!("   Expected: Accept N drafted → next iteration drafts from context+N. Instead: +1 always.");
    println!("\nB. Context Divergence: When draft diverges from target ground truth mid-sequence,");
    println!("   the correction token becomes the new context basis for next draft pass.");
    println!("   This compounds error if correction is speculative, not grounded.");
    println!("\nC. Infinite Stall: Empty draft results in immediate loop break, never reaching max_length.");
    println!("   No fallback to target-only generation.");
    println!("\nD. No Rejection Handling: If verify() returns correction without ever grounding it");
    println!("   against a real model pass, the sequence becomes hallucinated.");
}
