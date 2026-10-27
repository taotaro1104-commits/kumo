//! 角の丸い背景と枠線（`border-radius`）。角は円弧（縦横同じ半径）で近似し、アンチエイリアスで描く。

use kumo_css::LengthPct;
use kumo_css::values::Rgba;
use tiny_skia::{FillRule, Mask, Paint, Path, PathBuilder, Pixmap, Rect, Transform};

use crate::display_list::RectF;

/// 円弧を3次ベジェで近似するときの係数。
const KAPPA: f32 = 0.552_284_8;

/// 角の半径（左上・右上・右下・左下）を px にし、隣り合う角の和が辺の長さを超えないよう全体を縮める
/// （CSS の規定どおり）。すべて 0 なら `None`（四角のまま描く）。
pub fn resolve_radii(radii: &[LengthPct; 4], rect: RectF) -> Option<[f32; 4]> {
    let r = radii.map(|l| l.resolve(rect.w).max(0.0));
    if r.iter().all(|&v| v <= 0.0) || rect.w <= 0.0 || rect.h <= 0.0 {
        return None;
    }
    let scale = [
        rect.w / (r[0] + r[1]),
        rect.w / (r[3] + r[2]),
        rect.h / (r[0] + r[3]),
        rect.h / (r[1] + r[2]),
    ]
    .into_iter()
    .filter(|s| s.is_finite())
    .fold(1.0_f32, f32::min);
    Some(r.map(|v| v * scale))
}

/// 角丸の矩形のパス（時計回り）。
pub(crate) fn rounded_path(rect: RectF, radii: [f32; 4]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    push_rounded(&mut pb, rect, radii);
    pb.finish()
}

fn push_rounded(pb: &mut PathBuilder, rect: RectF, [tl, tr, br, bl]: [f32; 4]) {
    let (x0, y0, x1, y1) = (rect.x, rect.y, rect.right(), rect.bottom());
    let k = |r: f32| r * (1.0 - KAPPA);
    pb.move_to(x0 + tl, y0);
    pb.line_to(x1 - tr, y0);
    pb.cubic_to(x1 - k(tr), y0, x1, y0 + k(tr), x1, y0 + tr);
    pb.line_to(x1, y1 - br);
    pb.cubic_to(x1, y1 - k(br), x1 - k(br), y1, x1 - br, y1);
    pb.line_to(x0 + bl, y1);
    pb.cubic_to(x0 + k(bl), y1, x0, y1 - k(bl), x0, y1 - bl);
    pb.line_to(x0, y0 + tl);
    pb.cubic_to(x0, y0 + k(tl), x0 + k(tl), y0, x0 + tl, y0);
    pb.close();
}

fn paint(color: Rgba) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(tiny_skia::Color::from_rgba8(
        color.r, color.g, color.b, color.a,
    ));
    paint.anti_alias = true;
    paint
}

/// overflow のクリップ（矩形）をマスクにする。
fn clip_mask(out: &Pixmap, clip: Option<RectF>) -> Option<Mask> {
    let clip = clip?;
    let mut mask = Mask::new(out.width(), out.height())?;
    let rect = Rect::from_xywh(clip.x, clip.y, clip.w, clip.h)?;
    mask.fill_path(
        &PathBuilder::from_rect(rect),
        FillRule::Winding,
        false,
        Transform::identity(),
    );
    Some(mask)
}

/// 角丸の背景を塗る。
pub(crate) fn fill_rounded(
    out: &mut Pixmap,
    rect: RectF,
    color: Rgba,
    radii: [f32; 4],
    clip: Option<RectF>,
) {
    let Some(path) = rounded_path(rect, radii) else {
        return;
    };
    let mask = clip_mask(out, clip);
    out.fill_path(
        &path,
        &paint(color),
        FillRule::Winding,
        Transform::identity(),
        mask.as_ref(),
    );
}

/// 角丸の枠線を塗る（外側の角丸矩形から、枠線の幅だけ内側の角丸矩形を抜く）。
/// `widths` は上・右・下・左。内側の角の半径は外側の半径から隣り合う辺の太い方を引いたもの。
pub(crate) fn fill_rounded_border(
    out: &mut Pixmap,
    rect: RectF,
    widths: [f32; 4],
    color: Rgba,
    radii: [f32; 4],
    clip: Option<RectF>,
) {
    let [t, r, b, l] = widths;
    let inner = RectF::new(
        rect.x + l,
        rect.y + t,
        (rect.w - l - r).max(0.0),
        (rect.h - t - b).max(0.0),
    );
    let inner_radii = [
        (radii[0] - t.max(l)).max(0.0),
        (radii[1] - t.max(r)).max(0.0),
        (radii[2] - b.max(r)).max(0.0),
        (radii[3] - b.max(l)).max(0.0),
    ];
    let mut pb = PathBuilder::new();
    push_rounded(&mut pb, rect, radii);
    if inner.w > 0.0 && inner.h > 0.0 {
        push_rounded(&mut pb, inner, inner_radii);
    }
    let Some(path) = pb.finish() else { return };
    let mask = clip_mask(out, clip);
    out.fill_path(
        &path,
        &paint(color),
        FillRule::EvenOdd,
        Transform::identity(),
        mask.as_ref(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgba = Rgba::new(255, 0, 0, 255);

    fn canvas() -> Pixmap {
        let mut p = Pixmap::new(40, 40).unwrap();
        p.fill(tiny_skia::Color::WHITE);
        p
    }

    fn rgb(p: &Pixmap, x: u32, y: u32) -> (u8, u8, u8) {
        let c = p.pixel(x, y).unwrap();
        (c.red(), c.green(), c.blue())
    }

    #[test]
    fn radii_resolve_percentages_and_shrink_to_fit() {
        let rect = RectF::new(0.0, 0.0, 100.0, 40.0);
        let half = LengthPct { px: 0.0, pct: 50.0 };
        assert_eq!(resolve_radii(&[LengthPct::ZERO; 4], rect), None);
        // 50% は幅 100 の 50px だが、高さ 40 に収まるよう 20px に縮む（ピル型）
        assert_eq!(resolve_radii(&[half; 4], rect), Some([20.0; 4]));
        let small = [
            LengthPct::px(4.0),
            LengthPct::ZERO,
            LengthPct::ZERO,
            LengthPct::ZERO,
        ];
        assert_eq!(resolve_radii(&small, rect), Some([4.0, 0.0, 0.0, 0.0]));
    }

    #[test]
    fn rounded_fill_leaves_the_corners_empty() {
        let mut p = canvas();
        fill_rounded(
            &mut p,
            RectF::new(0.0, 0.0, 40.0, 40.0),
            RED,
            [20.0; 4],
            None,
        );
        assert_eq!(rgb(&p, 20, 20), (255, 0, 0), "中央は塗る");
        assert_eq!(rgb(&p, 1, 1), (255, 255, 255), "角は塗らない");
        assert_eq!(rgb(&p, 20, 1), (255, 0, 0), "辺の中央は塗る");
    }

    #[test]
    fn rounded_border_is_a_ring_and_respects_the_clip() {
        let mut p = canvas();
        let rect = RectF::new(0.0, 0.0, 40.0, 40.0);
        fill_rounded_border(&mut p, rect, [4.0; 4], RED, [10.0; 4], None);
        assert_eq!(rgb(&p, 20, 1), (255, 0, 0), "枠線");
        assert_eq!(rgb(&p, 20, 20), (255, 255, 255), "内側は抜く");
        assert_eq!(rgb(&p, 0, 0), (255, 255, 255), "角の外");
        let mut clipped = canvas();
        let clip = Some(RectF::new(0.0, 0.0, 20.0, 40.0));
        fill_rounded(&mut clipped, rect, RED, [5.0; 4], clip);
        assert_eq!(rgb(&clipped, 10, 20), (255, 0, 0));
        assert_eq!(
            rgb(&clipped, 30, 20),
            (255, 255, 255),
            "クリップの外は塗らない"
        );
    }
}
