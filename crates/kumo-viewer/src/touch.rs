//! タッチ操作（スマートフォン向け）：1本指のドラッグでスクロール、短いタップでクリック、
//! 2本指のつまみで拡大・縮小。座標は描画面の論理 px で受け取る。

/// タップとドラッグを分ける距離（論理 px）。これより動かなければタップ。
const TAP_SLOP: f64 = 10.0;
/// つまみの幅がこの倍率だけ変わるごとに、拡大・縮小を1段進める。
const PINCH_STEP: f64 = 1.25;

type Point = (f64, f64);

/// 指の動きから決まった操作。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gesture {
    None,
    /// ページを動かす量（px、下向きが正）
    Scroll(f32),
    Tap(f64, f64),
    ZoomIn,
    ZoomOut,
}

/// 画面に触れている指の状態。
#[derive(Default)]
pub struct Touches {
    points: Vec<(u64, Point)>,
    /// 最初の指が触れた場所
    start: Point,
    /// 1本指でスクロール量を求めるときの前回の位置
    last: Point,
    /// タップの許容を超えて動いた
    dragged: bool,
    /// この操作の間に2本以上が触れた（タップにしない）
    multi: bool,
    /// つまみの基準になる指の間隔
    pinch_base: f64,
}

fn distance(a: Point, b: Point) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

impl Touches {
    pub fn down(&mut self, id: u64, pos: Point) -> Gesture {
        self.points.retain(|(i, _)| *i != id);
        self.points.push((id, pos));
        match self.points.as_slice() {
            [_] => {
                self.start = pos;
                self.last = pos;
                self.dragged = false;
                self.multi = false;
            }
            [(_, a), (_, b)] => {
                self.multi = true;
                self.pinch_base = distance(*a, *b).max(1.0);
            }
            _ => {}
        }
        Gesture::None
    }

    pub fn moved(&mut self, id: u64, pos: Point) -> Gesture {
        let Some(slot) = self.points.iter_mut().find(|(i, _)| *i == id) else {
            return Gesture::None;
        };
        slot.1 = pos;
        match self.points.as_slice() {
            [(_, a), (_, b)] => self.pinch(distance(*a, *b)),
            [_] if !self.multi => self.drag(pos),
            _ => Gesture::None,
        }
    }

    fn drag(&mut self, pos: Point) -> Gesture {
        if !self.dragged && distance(self.start, pos) < TAP_SLOP {
            return Gesture::None;
        }
        self.dragged = true;
        let dy = pos.1 - self.last.1;
        self.last = pos;
        // 指を上へ動かすとページは下へ進む
        Gesture::Scroll(-dy as f32)
    }

    fn pinch(&mut self, now: f64) -> Gesture {
        let ratio = now / self.pinch_base;
        if ratio >= PINCH_STEP {
            self.pinch_base = now;
            Gesture::ZoomIn
        } else if ratio <= 1.0 / PINCH_STEP {
            self.pinch_base = now.max(1.0);
            Gesture::ZoomOut
        } else {
            Gesture::None
        }
    }

    /// 指が離れた。最後の1本が動かずに離れたらタップ。
    pub fn up(&mut self, id: u64, pos: Point) -> Gesture {
        self.points.retain(|(i, _)| *i != id);
        if self.points.is_empty() {
            let tap = !self.dragged && !self.multi;
            return if tap {
                Gesture::Tap(pos.0, pos.1)
            } else {
                Gesture::None
            };
        }
        if let [(_, rest)] = self.points.as_slice() {
            // つまみの後に残った指でスクロールが飛ばないよう、基準を取り直す
            self.last = *rest;
        }
        Gesture::None
    }

    /// システムが操作を取り消した（タップにはしない）。
    pub fn cancel(&mut self, id: u64) {
        self.points.retain(|(i, _)| *i != id);
        self.dragged = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_touch_is_a_tap() {
        let mut t = Touches::default();
        t.down(1, (100.0, 200.0));
        assert_eq!(t.moved(1, (103.0, 202.0)), Gesture::None);
        assert_eq!(t.up(1, (103.0, 202.0)), Gesture::Tap(103.0, 202.0));
    }

    #[test]
    fn dragging_scrolls_against_finger_direction() {
        let mut t = Touches::default();
        t.down(1, (50.0, 400.0));
        // 許容を超えた最初の動きは、触れた場所からの全距離
        assert_eq!(t.moved(1, (50.0, 380.0)), Gesture::Scroll(20.0));
        assert_eq!(t.moved(1, (50.0, 350.0)), Gesture::Scroll(30.0));
        assert_eq!(t.moved(1, (50.0, 360.0)), Gesture::Scroll(-10.0));
        assert_eq!(
            t.up(1, (50.0, 360.0)),
            Gesture::None,
            "ドラッグ後はタップにしない"
        );
    }

    #[test]
    fn small_wobble_never_scrolls() {
        let mut t = Touches::default();
        t.down(1, (10.0, 10.0));
        for dy in [2.0, -3.0, 5.0, -1.0] {
            assert_eq!(t.moved(1, (10.0, 10.0 + dy)), Gesture::None);
        }
    }

    #[test]
    fn pinch_zooms_one_step_per_ratio() {
        let mut t = Touches::default();
        t.down(1, (100.0, 300.0));
        t.down(2, (200.0, 300.0));
        assert_eq!(t.moved(2, (210.0, 300.0)), Gesture::None);
        assert_eq!(t.moved(2, (230.0, 300.0)), Gesture::ZoomIn);
        // 基準を取り直すので、同じ幅のままなら続けて拡大しない
        assert_eq!(t.moved(2, (230.0, 300.0)), Gesture::None);
        assert_eq!(t.moved(2, (160.0, 300.0)), Gesture::ZoomOut);
    }

    #[test]
    fn leftover_finger_after_pinch_does_not_tap_or_jump() {
        let mut t = Touches::default();
        t.down(1, (100.0, 300.0));
        t.down(2, (200.0, 300.0));
        assert_eq!(t.up(2, (200.0, 300.0)), Gesture::None);
        assert_eq!(t.moved(1, (100.0, 320.0)), Gesture::None);
        assert_eq!(t.up(1, (100.0, 320.0)), Gesture::None);
        // 次の操作は普通のタップになる
        t.down(3, (5.0, 5.0));
        assert_eq!(t.up(3, (5.0, 5.0)), Gesture::Tap(5.0, 5.0));
    }

    #[test]
    fn cancelled_touch_is_not_a_tap() {
        let mut t = Touches::default();
        t.down(1, (5.0, 5.0));
        t.cancel(1);
        assert_eq!(t.up(1, (5.0, 5.0)), Gesture::None);
    }

    #[test]
    fn unknown_finger_is_ignored() {
        let mut t = Touches::default();
        assert_eq!(t.moved(9, (1.0, 1.0)), Gesture::None);
    }
}
