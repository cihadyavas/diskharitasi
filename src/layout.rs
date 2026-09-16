use eframe::egui::{Rect, pos2};

use crate::tree::Tree;

/// Klasör başlık şeridinin yüksekliği ve iç boşluk (piksel).
pub const HEADER_H: f32 = 13.0;
pub const PAD: f32 = 3.0;
const MAX_ITEMS: usize = 300_000;

#[derive(Clone, Copy)]
pub struct Item {
    pub node: u32,
    pub rect: Rect,
    /// Klasör ve başlık şeridi çizilecek kadar yer var.
    pub header: bool,
}

pub struct Params {
    /// En küçük kutu kenarı; bundan küçükler çizilmez.
    pub min_px: f32,
    /// -4 (yatay kutular) … 0 (eşit) … +4 (dikey kutular)
    pub bias: i32,
}

/// `view` klasörünün çocuklarını `area` içine yerleştirir. Ebeveynler çocuklarından önce gelir.
pub fn build(tree: &Tree, view: u32, area: Rect, p: &Params) -> Vec<Item> {
    let mut out = Vec::new();
    let stretch = 2f32.powf(p.bias as f32 / 2.0);
    lay_children(tree, view, area, p, stretch, &mut out);
    out
}

fn lay_children(tree: &Tree, dir: u32, area: Rect, p: &Params, stretch: f32, out: &mut Vec<Item>) {
    let node = &tree.nodes[dir as usize];
    if node.size == 0 || area.width() < p.min_px || area.height() < p.min_px {
        return;
    }
    let children: Vec<u32> = node
        .children
        .iter()
        .copied()
        .filter(|&c| tree.nodes[c as usize].size > 0)
        .collect();
    if children.is_empty() {
        return;
    }
    let sizes: Vec<f64> = children.iter().map(|&c| tree.nodes[c as usize].size as f64).collect();
    let mut rects = Vec::with_capacity(sizes.len());
    squarify(&sizes, node.size as f64, area, stretch, &mut rects);

    for (&c, r) in children.iter().zip(rects) {
        if out.len() >= MAX_ITEMS {
            return;
        }
        let r = Rect::from_min_max(r.min.round(), r.max.round());
        if r.width() < p.min_px || r.height() < p.min_px {
            continue;
        }
        let child = &tree.nodes[c as usize];
        let header = child.is_dir && r.height() >= HEADER_H + 2.0 && r.width() >= 12.0;
        out.push(Item { node: c, rect: r, header });
        if header && !child.children.is_empty() {
            let inner = Rect::from_min_max(
                pos2(r.min.x + PAD, r.min.y + HEADER_H),
                pos2(r.max.x - PAD, r.max.y - PAD),
            );
            lay_children(tree, c, inner, p, stretch, out);
        }
    }
}

/// Squarified treemap (Bruls, Huizing, van Wijk). `sizes` büyükten küçüğe sıralı.
/// `total` toplamdan büyükse artan alan sağ altta boş kalır.
/// `stretch` > 1 kutuları dikeyleştirir, < 1 yataylaştırır.
fn squarify(sizes: &[f64], total: f64, area: Rect, stretch: f32, out: &mut Vec<Rect>) {
    let x0 = area.min.x as f64;
    let stretch = stretch as f64;
    let mut x = 0.0;
    let mut y = area.min.y as f64;
    let mut w = area.width() as f64 * stretch;
    let mut h = area.height() as f64;
    let scale = (w * h) / total.max(1.0);
    let unstretch = |sx: f64, sy: f64, ex: f64, ey: f64| {
        Rect::from_min_max(
            pos2((x0 + sx / stretch) as f32, sy as f32),
            pos2((x0 + ex / stretch) as f32, ey as f32),
        )
    };

    let mut i = 0;
    while i < sizes.len() {
        if w <= 0.0 || h <= 0.0 {
            break;
        }
        let side = w.min(h);
        let rmax = sizes[i] * scale;
        let mut sum = 0.0;
        let mut worst_prev = f64::INFINITY;
        let mut j = i;
        while j < sizes.len() {
            let a = sizes[j] * scale;
            let nsum = sum + a;
            let worst = (side * side * rmax / (nsum * nsum)).max(nsum * nsum / (side * side * a));
            if j > i && worst > worst_prev {
                break;
            }
            worst_prev = worst;
            sum = nsum;
            j += 1;
        }
        if w >= h {
            let cw = (sum / h).min(w);
            let mut yy = y;
            for &s in &sizes[i..j] {
                let hh = s * scale / cw;
                out.push(unstretch(x, yy, x + cw, yy + hh));
                yy += hh;
            }
            x += cw;
            w -= cw;
        } else {
            let rh = (sum / w).min(h);
            let mut xx = x;
            for &s in &sizes[i..j] {
                let ww = s * scale / rh;
                out.push(unstretch(xx, y, xx + ww, y + rh));
                xx += ww;
            }
            y += rh;
            h -= rh;
        }
        i = j;
    }
    // Sığmayanlar (yuvarlama artığı) boş kutu alır, sayılar eşleşsin.
    while out.len() < sizes.len() {
        out.push(Rect::NOTHING);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alanlar_orantili() {
        let sizes = [60.0, 30.0, 10.0];
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        let mut out = Vec::new();
        squarify(&sizes, 100.0, area, 1.0, &mut out);
        for (s, r) in sizes.iter().zip(&out) {
            let a = (r.width() * r.height()) as f64;
            assert!((a - s * 50.0).abs() < 1.0, "{a} != {}", s * 50.0);
            assert!(area.expand(0.01).contains_rect(*r));
        }
    }
}
