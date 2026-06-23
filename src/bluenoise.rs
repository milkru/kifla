//! Deterministic blue-noise tile generation via the void-and-cluster method
//! (Ulichney, 1993). Produces a `size`x`size` threshold texture whose values
//! are evenly dispersed in space and across all intensities, so tiling it gives
//! high-quality ordered dithering with no visible grid.

fn wrap_idx(x: i32, y: i32, size: i32) -> usize {
    (y.rem_euclid(size) * size + x.rem_euclid(size)) as usize
}

/// Add (or subtract, via `sign`) a Gaussian energy bump centered at pixel `p`,
/// wrapping at the tile edges so the pattern stays seamless.
fn splat(energy: &mut [f32], p: usize, size: i32, radius: i32, two_sig2: f32, sign: f32) {
    let px = (p as i32) % size;
    let py = (p as i32) / size;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let d2 = (dx * dx + dy * dy) as f32;
            let w = (-d2 / two_sig2).exp();
            energy[wrap_idx(px + dx, py + dy, size)] += sign * w;
        }
    }
}

fn tightest_cluster(pattern: &[bool], energy: &[f32]) -> usize {
    let mut best = 0;
    let mut best_e = f32::NEG_INFINITY;
    for i in 0..pattern.len() {
        if pattern[i] && energy[i] > best_e {
            best_e = energy[i];
            best = i;
        }
    }
    best
}

fn largest_void(pattern: &[bool], energy: &[f32]) -> usize {
    let mut best = 0;
    let mut best_e = f32::INFINITY;
    for i in 0..pattern.len() {
        if !pattern[i] && energy[i] < best_e {
            best_e = energy[i];
            best = i;
        }
    }
    best
}

/// Returns a `size`x`size` blue-noise tile as 8-bit thresholds (row-major).
pub fn generate(size: usize) -> Vec<u8> {
    let n = size * size;
    let si = size as i32;
    let sigma = 1.9_f32;
    let two_sig2 = 2.0 * sigma * sigma;
    let radius = (sigma * 3.0).ceil() as i32;

    // Deterministic LCG so the tile is identical every run.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rng = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as usize
    };

    // Seed ~10% of pixels as the initial minority set.
    let ones_target = (n / 10).max(1);
    let mut pattern = vec![false; n];
    let mut placed = 0;
    while placed < ones_target {
        let i = rng() % n;
        if !pattern[i] {
            pattern[i] = true;
            placed += 1;
        }
    }

    let mut energy = vec![0f32; n];
    for (i, &set) in pattern.iter().enumerate() {
        if set {
            splat(&mut energy, i, si, radius, two_sig2, 1.0);
        }
    }

    // Relax the seed pattern: repeatedly move the tightest cluster into the
    // largest void until that move is a no-op (the pattern is stable).
    loop {
        let c = tightest_cluster(&pattern, &energy);
        pattern[c] = false;
        splat(&mut energy, c, si, radius, two_sig2, -1.0);
        let v = largest_void(&pattern, &energy);
        pattern[v] = true;
        splat(&mut energy, v, si, radius, two_sig2, 1.0);
        if v == c {
            break;
        }
    }

    let ones = pattern.iter().filter(|&&b| b).count();
    let mut rank = vec![0u32; n];

    // Phase 1: remove ones from the tightest clusters; the removal order assigns
    // ranks (ones-1) down to 0.
    let mut work = pattern.clone();
    let mut e = energy.clone();
    for r in (0..ones).rev() {
        let c = tightest_cluster(&work, &e);
        rank[c] = r as u32;
        work[c] = false;
        splat(&mut e, c, si, radius, two_sig2, -1.0);
    }

    // Phase 2/3: fill the largest voids; ranks `ones` up to n-1.
    let mut work = pattern.clone();
    let mut e = energy.clone();
    for r in ones..n {
        let v = largest_void(&work, &e);
        rank[v] = r as u32;
        work[v] = true;
        splat(&mut e, v, si, radius, two_sig2, 1.0);
    }

    let denom = (n as f32 - 1.0).max(1.0);
    rank.iter()
        .map(|&r| ((r as f32 / denom) * 255.0).round() as u8)
        .collect()
}
