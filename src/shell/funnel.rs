// SPDX-License-Identifier: GPL-3.0-only

//! Funnel scale field: windows keep full size in the middle third of an
//! output and shrink uniformly (aspect ratio preserved) towards the edges.
//!
//! Shape (per output, left to right):  \\\\  ==  ==  ////

/// Scale a window has when its center sits directly on the output edge.
pub const MIN_SCALE: f64 = 0.12;

/// A shrunk window whose on-screen width drops below this many *logical* pixels
/// is treated as "too small to use": a left click anywhere on it moves it instead
/// of being passed to the client. Logical pixels already account for output
/// scaling, so this means the same visual size on every display.
pub const MOVE_ANYWHERE_WIDTH: f64 = 550.0;

/// ...but only once the window is noticeably shrunk, so windows that are small by
/// nature (dialogs, calculators) stay usable when they are only slightly scaled.
pub const MOVE_ANYWHERE_MAX_SCALE: f64 = 0.75;

/// Outline color for windows that can only be moved (RGB, 0..1).
pub const MOVE_ONLY_COLOR: [f32; 3] = [0.20, 0.52, 1.00];

/// Minimum outline thickness for move-only windows, even if the user disabled
/// the active-window hint.
pub const MOVE_ONLY_THICKNESS: u8 = 3;

/// Roadmap item 2 ("Docking"): a window dropped this small (or smaller) snaps
/// into a slot at the nearest edge instead of staying wherever it was dropped.
/// Same "visible size" measure as `MOVE_ANYWHERE_WIDTH`, just a smaller number,
/// so it only fires once the window is essentially at the edge already.
pub const DOCK_VISIBLE_WIDTH: f64 = 250.0;

/// Same guard as `MOVE_ANYWHERE_MAX_SCALE`: only windows that are actually deep
/// in the funnel's outer zone dock, not windows that are just small by nature.
pub const DOCK_MAX_SCALE: f64 = 0.5;

/// Gap in logical pixels between two docked windows stacked on the same edge.
pub const DOCK_GAP: i32 = 8;

/// Which output edge a docked window's slot belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSide {
    Left,
    Right,
}

/// Whether a window of logical width `native_w`, drawn at funnel scale `s`, should
/// be moved by a plain click anywhere on it.
pub fn is_move_anywhere(native_w: i32, s: f64) -> bool {
    s < MOVE_ANYWHERE_MAX_SCALE && native_w as f64 * s < MOVE_ANYWHERE_WIDTH
}

/// Whether a window of logical width `native_w`, dropped at funnel scale `s`,
/// should dock into an edge slot instead of staying where it was dropped.
pub fn is_dockable(native_w: i32, s: f64) -> bool {
    s < DOCK_MAX_SCALE && native_w as f64 * s < DOCK_VISIBLE_WIDTH
}

/// Render scale for a window whose center is at `x` (relative to the output's
/// left edge) on an output of width `w`. Returns 1.0 in the middle third and
/// falls off linearly to `MIN_SCALE` at both edges.
pub fn scale_at(x: f64, w: f64) -> f64 {
    if w <= 0.0 {
        return 1.0;
    }
    let t = (x / w).clamp(0.0, 1.0);
    let d = if t < 1.0 / 3.0 {
        (1.0 / 3.0 - t) * 3.0
    } else if t > 2.0 / 3.0 {
        (t - 2.0 / 3.0) * 3.0
    } else {
        0.0
    };
    1.0 - (1.0 - MIN_SCALE) * d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middle_third_is_unscaled() {
        assert_eq!(scale_at(1280.0, 2560.0), 1.0);
        assert_eq!(scale_at(900.0, 2560.0), 1.0);
    }

    #[test]
    fn edges_hit_min_scale() {
        assert!((scale_at(0.0, 2560.0) - MIN_SCALE).abs() < 1e-9);
        assert!((scale_at(2560.0, 2560.0) - MIN_SCALE).abs() < 1e-9);
    }

    #[test]
    fn move_anywhere() {
        assert!(!is_move_anywhere(1600, 0.4)); // 640 px, still usable
        assert!(is_move_anywhere(1600, 0.3)); // 480 px
        assert!(!is_move_anywhere(450, 0.9)); // small by nature, barely shrunk
        assert!(is_move_anywhere(450, 0.7));
    }

    #[test]
    fn dockable() {
        assert!(!is_dockable(1600, 0.4)); // 640 px, not near the edge yet
        assert!(is_dockable(1600, MIN_SCALE)); // ~192 px, right at the edge
        assert!(!is_dockable(450, 0.9)); // small by nature, barely shrunk
    }

    #[test]
    fn dock_implies_move_anywhere() {
        // Anything small enough to dock is also small enough to be move-only;
        // a window should never be draggable-through *and* dockable at once
        // in a way that leaves the click-passthrough behavior undefined.
        for w in [400, 900, 1600, 2200] {
            for i in 0..=100 {
                let s = MIN_SCALE + (1.0 - MIN_SCALE) * (i as f64 / 100.0);
                if is_dockable(w, s) {
                    assert!(is_move_anywhere(w, s), "w={w} s={s}");
                }
            }
        }
    }

    #[test]
    fn symmetric() {
        assert!((scale_at(300.0, 2560.0) - scale_at(2260.0, 2560.0)).abs() < 1e-9);
    }
}

/// Start every workspace in floating mode, regardless of the user's autotile
/// setting. The nested prototype shares ~/.config/cosmic with the real session,
/// so we must not flip the setting itself. Enabled by `COSMIC_FUNNEL_FLOATING=1`.
pub fn force_floating() -> bool {
    std::env::var_os("COSMIC_FUNNEL_FLOATING").is_some_and(|v| v != "0")
}

// ---------------------------------------------------------------------------
// Input mapping
//
// Smithay's pointer focus only knows a translation: the surface-local position
// it hands to a client is `pointer - focus_location`. For a window rendered at
// scale `s`, `surface_under` returns the *scaled* on-screen origin of the
// surface, so `pointer - origin` is `s` times too small/large. We remember the
// scale of the most recent hit here and divide it out again in
// `PointerFocusTarget::{enter, motion}`.
//
// Prototype simplification: one global value, reset at the start of every
// `State::surface_under`. During an implicit click-grab the value can briefly
// belong to a different window than the grabbed one.
// ---------------------------------------------------------------------------

use smithay::input::pointer::MotionEvent;
use std::sync::atomic::{AtomicU64, Ordering};

static POINTER_SCALE: AtomicU64 = AtomicU64::new(0x3FF0_0000_0000_0000); // 1.0_f64

pub fn set_pointer_scale(s: f64) {
    POINTER_SCALE.store(s.to_bits(), Ordering::Relaxed);
}

pub fn pointer_scale() -> f64 {
    f64::from_bits(POINTER_SCALE.load(Ordering::Relaxed))
}

/// Convert a surface-local motion event from on-screen (scaled) units back
/// into the client's own coordinate space.
pub fn unscale_motion(event: &MotionEvent) -> MotionEvent {
    let s = pointer_scale();
    let location = if (s - 1.0).abs() < 1e-6 {
        event.location
    } else {
        event.location.downscale(s)
    };
    MotionEvent {
        location,
        serial: event.serial,
        time: event.time,
    }
}
