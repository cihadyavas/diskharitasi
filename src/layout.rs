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
    /// SpaceMonger 1.4 tarzı ikiye bölme yerleşimi.
    pub classic: bool,
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
    if p.classic {
        rects.resize(sizes.len(), Rect::NOTHING);
        let idx: Vec<usize> = (0..sizes.len()).collect();
        let a = Rect::from_min_max(area.min.floor(), area.max.floor());
        classic_split(&sizes, &idx, a, p.bias, p.min_px, &mut rects);
    } else {
        squarify(&sizes, node.size as f64, area, stretch, &mut rects);
    }

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

/// SpaceMonger 1.4'ün yerleşimi: büyükten küçüğe sıralı öğeleri toplamları dengeli iki
/// listeye dağıt, alanı (eğilime göre) uzun kenardan orantılı kes, her yarıda tekrarla.
/// Uyarlandığı kaynak: https://github.com/seanofw/spacemonger1 (FolderView.cpp, SizeFolders)
/// Copyright (c) 1998-2020 Sean Werkema, MIT License.
fn classic_split(sizes: &[f64], idx: &[usize], r: Rect, bias: i32, min_px: f32, out: &mut [Rect]) {
    let (mut l1, mut l2) = (Vec::new(), Vec::new());
    let (mut s1, mut s2) = (0.0, 0.0);
    for &i in idx {
        if sizes[i] > 0.0 {
            if s1 <= s2 {
                l1.push(i);
                s1 += sizes[i];
            } else {
                l2.push(i);
                s2 += sizes[i];
            }
        }
    }
    if s1 + s2 <= 0.0 {
        return;
    }
    let (wb, hb) = match bias {
        b if b > 0 => (b as f32 + 8.0, 8.0),
        b if b < 0 => (8.0, -b as f32 + 8.0),
        _ => (8.0, 8.0),
    };
    let (w, h) = (r.width(), r.height());
    let (r1, r2) = if w * wb > h * hb {
        let split = (w as f64 * s1 / (s1 + s2)).floor() as f32;
        let x = r.min.x + split;
        (Rect::from_min_max(r.min, pos2(x, r.max.y)), Rect::from_min_max(pos2(x, r.min.y), r.max))
    } else {
        let split = (h as f64 * s1 / (s1 + s2)).floor() as f32;
        let y = r.min.y + split;
        (Rect::from_min_max(r.min, pos2(r.max.x, y)), Rect::from_min_max(pos2(r.min.x, y), r.max))
    };
    for (list, rr) in [(l1, r1), (l2, r2)] {
        if list.is_empty() || rr.width() <= min_px || rr.height() <= min_px {
            continue;
        }
        if list.len() > 1 {
            classic_split(sizes, &list, rr, bias, min_px, out);
        } else {
            out[list[0]] = rr;
        }
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

    #[test]
    fn klasik_alan_orantili_ve_ortusmez() {
        let sizes = [50.0, 25.0, 15.0, 10.0];
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 100.0));
        let mut out = vec![Rect::NOTHING; 4];
        classic_split(&sizes, &[0, 1, 2, 3], area, 0, 1.0, &mut out);
        for (s, r) in sizes.iter().zip(&out) {
            let a = (r.width() * r.height()) as f64;
            assert!((a - s * 200.0).abs() < 250.0, "{a} != {}", s * 200.0);
        }
        for i in 0..4 {
            for j in i + 1..4 {
                assert!(out[i].intersect(out[j]).area() <= 0.0);
            }
        }
    }
}
