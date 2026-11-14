//! Icône de l'application (ancre cyan sur fond marine), dessinée par calcul.
//! Partagée entre le programme (icône de fenêtre) et build.rs (icône du .exe).

fn smooth(d: f32, aa: f32) -> f32 {
    (0.5 - d / aa).clamp(0.0, 1.0)
}

fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (vx, vy) = (bx - ax, by - ay);
    let (wx, wy) = (px - ax, py - ay);
    let t = ((wx * vx + wy * vy) / (vx * vx + vy * vy)).clamp(0.0, 1.0);
    let (dx, dy) = (wx - t * vx, wy - t * vy);
    (dx * dx + dy * dy).sqrt()
}

/// Pixels RGBA (non prémultipliés) d'une icône carrée de `size` pixels.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let s = size as f32;
    let aa = 1.5 / s;
    let mut out = vec![0u8; (size * size * 4) as usize];
    let bg = [0x0b as f32, 0x1d as f32, 0x33 as f32];
    let cy = [0x22 as f32, 0xd3 as f32, 0xee as f32];
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / s;
            let v = (y as f32 + 0.5) / s;
            // Carré arrondi
            let r = 0.22;
            let qx = (u - 0.5).abs() - (0.5 - r);
            let qy = (v - 0.5).abs() - (0.5 - r);
            let d_box = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r;
            let a_bg = smooth(d_box, aa);
            let border = smooth(d_box.abs() - 0.012, aa) * 0.55;
            // Ancre : anneau, tige, barre, arc et pointes.
            let w = 0.042;
            let ring = ((u - 0.5).hypot(v - 0.2) - 0.075).abs() - w * 0.8;
            let shank = seg_dist(u, v, 0.5, 0.28, 0.5, 0.82) - w;
            let bar = seg_dist(u, v, 0.33, 0.38, 0.67, 0.38) - w * 0.9;
            let arc_r = 0.3;
            let da = (u - 0.5).hypot(v - 0.5) - arc_r;
            let arc = if v > 0.58 { da.abs() - w } else { 1.0 };
            let tip_l = seg_dist(u, v, 0.235, 0.64, 0.17, 0.56) - w * 0.85;
            let tip_l2 = seg_dist(u, v, 0.235, 0.64, 0.30, 0.60) - w * 0.85;
            let tip_r = seg_dist(u, v, 0.765, 0.64, 0.83, 0.56) - w * 0.85;
            let tip_r2 = seg_dist(u, v, 0.765, 0.64, 0.70, 0.60) - w * 0.85;
            let d = ring.min(shank).min(bar).min(arc).min(tip_l).min(tip_l2).min(tip_r).min(tip_r2);
            let a_fg = smooth(d, aa) * a_bg;
            let mut c = [0f32; 3];
            for i in 0..3 {
                let base = bg[i] * (1.0 - border) + cy[i] * border;
                c[i] = base * (1.0 - a_fg) + cy[i] * a_fg;
            }
            let o = ((y * size + x) * 4) as usize;
            out[o] = c[0] as u8;
            out[o + 1] = c[1] as u8;
            out[o + 2] = c[2] as u8;
            out[o + 3] = (a_bg * 255.0) as u8;
        }
    }
    out
}
