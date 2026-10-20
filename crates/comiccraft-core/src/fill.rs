//! Region-finding algorithms shared by the Fill tool, the Magic Wand (Auto Select) and
//! the Close-and-Fill (Enclose and fill) tool.
//!
//! All functions work on an RGBA reference buffer of `w * h` pixels and return a
//! per-pixel boolean mask.

use std::collections::VecDeque;

/// Mean absolute RGBA difference (0..=255), same metric the original bucket used.
#[inline(always)]
pub fn color_diff(c1: [u8; 4], c2: [u8; 4]) -> u32 {
    let dr = (c1[0] as i32 - c2[0] as i32).unsigned_abs();
    let dg = (c1[1] as i32 - c2[1] as i32).unsigned_abs();
    let db = (c1[2] as i32 - c2[2] as i32).unsigned_abs();
    let da = (c1[3] as i32 - c2[3] as i32).unsigned_abs();
    (dr + dg + db + da) / 4
}

#[inline(always)]
fn px(buf: &[u8], i: usize) -> [u8; 4] {
    [buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]]
}

/// Chamfer (3-4) distance transform: distance (in ~pixels) from every pixel to the nearest
/// `true` pixel of `src`. Two passes, O(n).
pub fn distance_to(src: &[bool], w: usize, h: usize) -> Vec<u32> {
    const INF: u32 = u32::MAX / 4;
    let mut d: Vec<u32> = src.iter().map(|&b| if b { 0 } else { INF }).collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut v = d[i];
            if x > 0 {
                v = v.min(d[i - 1] + 3);
            }
            if y > 0 {
                v = v.min(d[i - w] + 3);
                if x > 0 {
                    v = v.min(d[i - w - 1] + 4);
                }
                if x + 1 < w {
                    v = v.min(d[i - w + 1] + 4);
                }
            }
            d[i] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut v = d[i];
            if x + 1 < w {
                v = v.min(d[i + 1] + 3);
            }
            if y + 1 < h {
                v = v.min(d[i + w] + 3);
                if x + 1 < w {
                    v = v.min(d[i + w + 1] + 4);
                }
                if x > 0 {
                    v = v.min(d[i + w - 1] + 4);
                }
            }
            d[i] = v;
        }
    }
    d
}

/// Grows (`radius > 0`) or shrinks (`radius < 0`) a mask by roughly `radius` pixels.
pub fn scale_mask(mask: &[bool], w: usize, h: usize, radius: i32) -> Vec<bool> {
    if radius == 0 {
        return mask.to_vec();
    }
    if radius > 0 {
        let d = distance_to(mask, w, h);
        let lim = radius as u32 * 3;
        d.iter().map(|&v| v <= lim).collect()
    } else {
        let inv: Vec<bool> = mask.iter().map(|b| !b).collect();
        let d = distance_to(&inv, w, h);
        let lim = (-radius) as u32 * 3;
        d.iter().map(|&v| v > lim).collect()
    }
}

/// 4-connected flood over `passable`, starting at `seed`.
pub fn flood(passable: &[bool], w: usize, h: usize, seed: usize) -> Vec<bool> {
    let mut out = vec![false; w * h];
    if seed >= w * h || !passable[seed] {
        return out;
    }
    let mut q = VecDeque::new();
    out[seed] = true;
    q.push_back(seed);
    while let Some(i) = q.pop_front() {
        let x = i % w;
        let y = i / w;
        let mut visit = |j: usize| {
            if passable[j] && !out[j] {
                out[j] = true;
                q.push_back(j);
            }
        };
        if x > 0 {
            visit(i - 1);
        }
        if x + 1 < w {
            visit(i + 1);
        }
        if y > 0 {
            visit(i - w);
        }
        if y + 1 < h {
            visit(i + w);
        }
    }
    out
}

/// Options shared by the bucket and the magic wand.
#[derive(Clone, Copy, Debug)]
pub struct RegionOptions {
    /// Color tolerance 0..=255.
    pub tolerance: u8,
    /// CSP "Close gap": gaps in lines up to ~this many pixels wide are treated as closed.
    pub close_gap: u32,
    /// CSP "Area scaling": grow (+) or shrink (-) the result by this many pixels.
    pub area_scale: i32,
    /// When false, selects every pixel of similar color, not only the connected region.
    pub contiguous: bool,
}

impl Default for RegionOptions {
    fn default() -> Self {
        Self { tolerance: 15, close_gap: 0, area_scale: 0, contiguous: true }
    }
}

/// Finds the region a bucket / magic wand click at `(sx, sy)` refers to.
pub fn region_mask(reference: &[u8], w: usize, h: usize, sx: usize, sy: usize, opt: RegionOptions) -> Vec<bool> {
    if sx >= w || sy >= h || reference.len() < w * h * 4 {
        return vec![false; w * h];
    }
    let seed = sy * w + sx;
    let target = px(reference, seed);
    let tol = opt.tolerance as u32;
    let passable: Vec<bool> = (0..w * h).map(|i| color_diff(px(reference, i), target) <= tol).collect();

    let mut region = if !opt.contiguous {
        passable.clone()
    } else if opt.close_gap > 0 {
        // Treat thin openings as walls: dilate the barrier, flood, then let the result
        // flow back over the pixels the dilation took away (but never across real lines).
        let barrier: Vec<bool> = passable.iter().map(|b| !b).collect();
        let dist = distance_to(&barrier, w, h);
        let lim = opt.close_gap as u32 * 3;
        let narrowed: Vec<bool> = dist.iter().map(|&d| d > lim).collect();
        let core = if narrowed[seed] { flood(&narrowed, w, h, seed) } else { flood(&passable, w, h, seed) };
        // Grow back within the original passable area, limited to the gap radius.
        let grow = distance_to(&core, w, h);
        let glim = (opt.close_gap as u32 + 1) * 3;
        (0..w * h).map(|i| core[i] || (passable[i] && grow[i] <= glim)).collect()
    } else {
        flood(&passable, w, h, seed)
    };

    if opt.area_scale != 0 {
        region = scale_mask(&region, w, h, opt.area_scale);
    }
    region
}

/// Rasterizes a polygon (even-odd rule) into a mask.
pub fn polygon_mask(poly: &[(f32, f32)], w: usize, h: usize) -> Vec<bool> {
    let mut out = vec![false; w * h];
    if poly.len() < 3 {
        return out;
    }
    let mut xs: Vec<f32> = Vec::with_capacity(16);
    for y in 0..h {
        let fy = y as f32 + 0.5;
        xs.clear();
        for i in 0..poly.len() {
            let (x1, y1) = poly[i];
            let (x2, y2) = poly[(i + 1) % poly.len()];
            if (y1 <= fy && y2 > fy) || (y2 <= fy && y1 > fy) {
                xs.push(x1 + (fy - y1) / (y2 - y1) * (x2 - x1));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in xs.chunks_exact(2) {
            let a = (pair[0] - 0.5).ceil().max(0.0) as usize;
            let b = ((pair[1] - 0.5).floor() as isize).min(w as isize - 1);
            if b < 0 {
                continue;
            }
            for x in a..=(b as usize) {
                out[y * w + x] = true;
            }
        }
    }
    out
}

/// Decides which reference pixels count as "line art" for Close-and-Fill.
/// Opaque-enough pixels that are noticeably darker than paper are lines.
pub fn line_barrier(reference: &[u8], w: usize, h: usize, active_layer_only: bool) -> Vec<bool> {
    (0..w * h)
        .map(|i| {
            let p = px(reference, i);
            if active_layer_only {
                p[3] > 127
            } else {
                let luma = (77 * p[0] as u32 + 150 * p[1] as u32 + 29 * p[2] as u32) >> 8;
                p[3] > 127 && luma < 160
            }
        })
        .collect()
}

/// CSP "Close and fill" (Enclose and fill): every area inside the lasso that is fully
/// enclosed by lines (does not touch the lasso edge) gets filled, then the fill is
/// expanded under the lines by `area_scale` pixels.
pub fn enclose_fill_mask(
    barrier: &[bool],
    w: usize,
    h: usize,
    poly: &[(f32, f32)],
    close_gap: u32,
    area_scale: i32,
) -> Vec<bool> {
    let inside = polygon_mask(poly, w, h);
    // Optionally thicken lines to close small gaps.
    let walls = if close_gap > 0 { scale_mask(barrier, w, h, close_gap as i32) } else { barrier.to_vec() };
    let open: Vec<bool> = (0..w * h).map(|i| inside[i] && !walls[i]).collect();

    // Flood from every open pixel that touches the outside of the lasso: those leak.
    let mut leak = vec![false; w * h];
    let mut q = VecDeque::new();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !open[i] {
                continue;
            }
            let at_edge = x == 0
                || y == 0
                || x + 1 == w
                || y + 1 == h
                || !inside[i - 1]
                || !inside[i + 1]
                || !inside[i - w]
                || !inside[i + w];
            if at_edge {
                leak[i] = true;
                q.push_back(i);
            }
        }
    }
    while let Some(i) = q.pop_front() {
        let x = i % w;
        let y = i / w;
        for j in [
            if x > 0 { Some(i - 1) } else { None },
            if x + 1 < w { Some(i + 1) } else { None },
            if y > 0 { Some(i - w) } else { None },
            if y + 1 < h { Some(i + w) } else { None },
        ]
        .into_iter()
        .flatten()
        {
            if open[j] && !leak[j] {
                leak[j] = true;
                q.push_back(j);
            }
        }
    }
    let mut fill: Vec<bool> = (0..w * h).map(|i| open[i] && !leak[i]).collect();
    if close_gap > 0 {
        // Give back the pixels the wall-thickening ate (but stay off real lines).
        let grown = scale_mask(&fill, w, h, close_gap as i32 + 1);
        fill = (0..w * h).map(|i| fill[i] || (grown[i] && inside[i] && !barrier[i])).collect();
    }
    if area_scale > 0 {
        let grown = scale_mask(&fill, w, h, area_scale);
        fill = (0..w * h).map(|i| grown[i] && inside[i]).collect();
    }
    fill
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 20x20 white canvas with a black square outline from (5,5) to (14,14) and a
    /// 1px gap in the top edge at x=10.
    fn ring(gap: bool) -> Vec<u8> {
        let (w, h) = (20usize, 20usize);
        let mut buf = vec![255u8; w * h * 4];
        for y in 5..=14 {
            for x in 5..=14 {
                if x == 5 || x == 14 || y == 5 || y == 14 {
                    if gap && y == 5 && x == 10 {
                        continue;
                    }
                    let i = (y * w + x) * 4;
                    buf[i..i + 3].copy_from_slice(&[0, 0, 0]);
                }
            }
        }
        buf
    }

    #[test]
    fn bucket_stays_inside_closed_ring() {
        let r = region_mask(&ring(false), 20, 20, 10, 10, RegionOptions::default());
        assert!(r[10 * 20 + 10]);
        assert!(!r[0]);
        assert_eq!(r.iter().filter(|b| **b).count(), 64);
    }

    #[test]
    fn gap_leaks_without_close_gap_and_holds_with_it() {
        let leaky = region_mask(&ring(true), 20, 20, 10, 10, RegionOptions::default());
        assert!(leaky[0], "1px gap should leak with close_gap = 0");
        let opt = RegionOptions { close_gap: 2, ..Default::default() };
        let held = region_mask(&ring(true), 20, 20, 10, 10, opt);
        assert!(!held[0], "close_gap should stop the leak");
        assert!(held[10 * 20 + 10]);
    }

    #[test]
    fn area_scaling_grows_under_lines() {
        let opt = RegionOptions { area_scale: 1, ..Default::default() };
        let r = region_mask(&ring(false), 20, 20, 10, 10, opt);
        assert!(r[5 * 20 + 10], "fill should extend 1px under the line");
        assert!(!r[3 * 20 + 10]);
    }

    #[test]
    fn non_contiguous_wand_selects_all_matching() {
        let opt = RegionOptions { contiguous: false, ..Default::default() };
        let r = region_mask(&ring(false), 20, 20, 10, 10, opt);
        assert!(r[0] && r[10 * 20 + 10]);
        assert!(!r[5 * 20 + 5]);
    }

    #[test]
    fn enclose_fill_fills_only_enclosed_area() {
        let buf = ring(false);
        let barrier = line_barrier(&buf, 20, 20, false);
        let poly = [(1.0, 1.0), (19.0, 1.0), (19.0, 19.0), (1.0, 19.0)];
        let f = enclose_fill_mask(&barrier, 20, 20, &poly, 0, 0);
        assert!(f[10 * 20 + 10]);
        assert!(!f[2 * 20 + 2], "area outside the ring touches the lasso edge");
        assert!(!f[5 * 20 + 5], "lines are not filled");
    }
}
