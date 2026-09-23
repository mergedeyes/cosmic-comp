// SPDX-License-Identifier: GPL-3.0-only

//! Funnel scale field: windows keep full size in the middle third of an
//! output and shrink uniformly (aspect ratio preserved) towards the edges.
//!
//! Shape (per output, left to right):  \\\\  ==  ==  ////

/// Pure safety clamp on any computed funnel scale -- never a design target
/// itself (that's `dock_scale`), just a floor so a pathological window/output
/// combination can't shrink a window to nothing.
pub const ABS_MIN_SCALE: f64 = 0.02;

/// Funnel edge-falloff progress below which a window is "too small to use":
/// a left click anywhere on it moves it instead of being passed to the
/// client. This is a position measure (see `falloff_at`) -- 0 at the
/// boundary of the middle third, 1 at the true output edge -- deliberately
/// *not* a raw scale number, because scale is now per-window (`dock_scale`);
/// comparing raw scale against one fixed threshold would mean some window
/// sizes never cross it at all (see `dock_scale`'s docs). Falloff progress
/// stays a pure fraction of position, so the same threshold means the same
/// thing for every window, at any resolution.
pub const MOVE_ANYWHERE_FALLOFF: f64 = 0.6;

/// Outline color for windows that can only be moved (RGB, 0..1).
pub const MOVE_ONLY_COLOR: [f32; 3] = [0.20, 0.52, 1.00];

/// Minimum outline thickness for move-only windows, even if the user disabled
/// the active-window hint.
pub const MOVE_ONLY_THICKNESS: u8 = 3;

/// Outline color for windows that are actually docked (RGB, 0..1) -- grey
/// rather than the move-only blue, so a resting slot in the dock reads as a
/// different, calmer state than "shrunk and click-through somewhere in the
/// funnel's outer zone."
pub const DOCKED_COLOR: [f32; 3] = [0.55, 0.55, 0.55];

/// Outline thickness for docked windows -- thinner than the ordinary
/// move-only outline, since a whole stack of docked slots sitting right next
/// to each other reads as busier than a single shrunk window elsewhere.
pub const DOCKED_THICKNESS: u8 = 2;

/// Roadmap item 2 ("Docking"): a window dropped at this much edge-falloff
/// progress (or more) snaps into a slot at the nearest edge instead of
/// staying wherever it was dropped. Higher than `MOVE_ANYWHERE_FALLOFF`, so
/// it only fires once the window is essentially at the edge already, not
/// just move-only.
pub const DOCK_FALLOFF: f64 = 0.85;

/// A docked window's slot size is decoupled from the funnel's edge-falloff
/// scale entirely -- it's a fixed size relative to the *output height*,
/// independent of both the window's native size and the fraction of the
/// output width it happens to be dragged into. This is the cap, as a
/// fraction of output height, on the *longer* of the slot's two dimensions:
/// a landscape window is capped by width, a portrait window by height, and
/// a square window by both at once (they're equal, so either branch of
/// `dock_scale` gives the same answer). The shorter dimension follows to
/// preserve the window's own aspect ratio. The result: the same number of
/// slots always fits along an edge, however wide/tall the monitor and
/// whatever shape the docked windows themselves are.
pub const DOCK_MAX_SIZE_FRACTION: f64 = 0.16;

/// Padding around docked slots, as a fraction of `dock_max_dim` -- so it
/// scales with the slot size instead of being a fixed pixel count that would
/// look wrong at very different `DOCK_MAX_SIZE_FRACTION` settings. Used for
/// three things, all the same value: the gap between a docked window's outer
/// edge and the screen edge, the inset from the top of the output for the
/// first slot in a stack, and the gap between two stacked slots.
pub const DOCK_PADDING_FRACTION: f64 = 0.15;

/// The longer-edge cap, in logical pixels, a docked slot is sized against on
/// an output of logical height `output_h`.
pub fn dock_max_dim(output_h: f64) -> f64 {
    output_h * DOCK_MAX_SIZE_FRACTION
}

/// Gap from the screen edge, in logical pixels, for a docked slot on an
/// output of logical height `output_h`.
pub fn dock_padding(output_h: f64) -> f64 {
    dock_max_dim(output_h) * DOCK_PADDING_FRACTION
}

/// The scale a window of native size `native_w` x `native_h` is drawn at
/// once fully docked/at the true edge, on an output of logical height
/// `output_h`: the longer of the two native dimensions is scaled down to
/// `dock_max_dim`, the other follows along to keep the window's own aspect
/// ratio. This is also the scale a live drag bottoms out at when it reaches
/// the edge (see `scale_at`'s `min_scale` parameter) -- one formula for both,
/// so a window never jumps size the moment it docks or the moment it's
/// picked back up. Clamped to never *enlarge* a window that's already
/// smaller than the slot cap (a small dialog just doesn't shrink further),
/// and never collapse to nothing for a degenerate size.
pub fn dock_scale(native_w: f64, native_h: f64, output_h: f64) -> f64 {
    let longer = native_w.max(native_h);
    if longer <= 0.0 {
        return ABS_MIN_SCALE;
    }
    (dock_max_dim(output_h) / longer).clamp(ABS_MIN_SCALE, 1.0)
}

/// Which output edge a docked window's slot belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSide {
    Left,
    Right,
}

/// Whether a window at this much edge-falloff progress (see `falloff_at`)
/// should be moved by a plain click anywhere on it, rather than passing the
/// click through.
pub fn is_move_anywhere(d: f64) -> bool {
    d >= MOVE_ANYWHERE_FALLOFF
}

/// Whether a window dropped at this much edge-falloff progress should dock
/// into an edge slot instead of staying where it was dropped.
pub fn is_dockable(d: f64) -> bool {
    d >= DOCK_FALLOFF
}

/// How far a position at `x` (relative to the output's left edge) on an
/// output of width `w` has fallen into the outer third: 0 anywhere in the
/// middle third, rising linearly to 1 at the output edge. Pure position
/// math -- independent of any window's size, aspect ratio, or scale.
pub fn falloff_at(x: f64, w: f64) -> f64 {
    if w <= 0.0 {
        return 0.0;
    }
    let t = (x / w).clamp(0.0, 1.0);
    if t < 1.0 / 3.0 {
        (1.0 / 3.0 - t) * 3.0
    } else if t > 2.0 / 3.0 {
        (t - 2.0 / 3.0) * 3.0
    } else {
        0.0
    }
}

/// Render scale for a window whose center is at `x` (relative to the
/// output's left edge) on an output of width `w`, given the scale it should
/// have fully at the edge (`min_scale` -- pass `dock_scale` for the window in
/// question, so a live drag bottoms out at exactly its docked size). Returns
/// 1.0 in the middle third and falls off linearly to `min_scale` at both
/// edges.
pub fn scale_at(x: f64, w: f64, min_scale: f64) -> f64 {
    1.0 - (1.0 - min_scale) * falloff_at(x, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middle_third_is_unscaled() {
        assert_eq!(scale_at(1280.0, 2560.0, 0.12), 1.0);
        assert_eq!(scale_at(900.0, 2560.0, 0.12), 1.0);
    }

    #[test]
    fn edges_hit_min_scale() {
        assert!((scale_at(0.0, 2560.0, 0.12) - 0.12).abs() < 1e-9);
        assert!((scale_at(2560.0, 2560.0, 0.12) - 0.12).abs() < 1e-9);
    }

    #[test]
    fn falloff_matches_scale_shape() {
        assert_eq!(falloff_at(1280.0, 2560.0), 0.0);
        assert_eq!(falloff_at(0.0, 2560.0), 1.0);
        assert_eq!(falloff_at(2560.0, 2560.0), 1.0);
    }

    #[test]
    fn move_anywhere() {
        assert!(!is_move_anywhere(0.5)); // not yet 60% into the outer third
        assert!(is_move_anywhere(0.7)); // past the threshold
        assert!(is_move_anywhere(MOVE_ANYWHERE_FALLOFF)); // boundary is inclusive
    }

    #[test]
    fn dockable() {
        assert!(!is_dockable(0.7)); // move-only already, but not deep enough to dock
        assert!(is_dockable(1.0)); // right at the edge
    }

    #[test]
    fn dock_implies_move_anywhere() {
        // Anything with enough edge-falloff progress to dock also has enough
        // to be move-only; a window should never be draggable-through *and*
        // dockable at once in a way that leaves click-passthrough undefined.
        // This is a constant-order relationship now (both are pure falloff
        // thresholds), but keep the sweep as a regression guard.
        for i in 0..=100 {
            let d = i as f64 / 100.0;
            if is_dockable(d) {
                assert!(is_move_anywhere(d), "d={d}");
            }
        }
    }

    #[test]
    fn dock_scale_caps_longer_edge() {
        // Landscape: width is the longer edge, gets capped; height follows.
        let s = dock_scale(1920.0, 1080.0, 1440.0);
        let max_dim = dock_max_dim(1440.0);
        assert!((1920.0 * s - max_dim).abs() < 1e-6);
        assert!(1080.0 * s < max_dim);

        // Portrait: height is the longer edge.
        let s = dock_scale(600.0, 1200.0, 1440.0);
        let max_dim = dock_max_dim(1440.0);
        assert!((1200.0 * s - max_dim).abs() < 1e-6);
        assert!(600.0 * s < max_dim);

        // Square: both edges hit the cap together.
        let s = dock_scale(800.0, 800.0, 1440.0);
        let max_dim = dock_max_dim(1440.0);
        assert!((800.0 * s - max_dim).abs() < 1e-6);
    }

    #[test]
    fn dock_scale_independent_of_output_width() {
        // The whole point: only output *height* and the window's own aspect
        // ratio should matter, never the output width or the window's
        // absolute size beyond its ratio.
        assert_eq!(dock_scale(1920.0, 1080.0, 1440.0), dock_scale(3840.0, 2160.0, 1440.0));
    }

    #[test]
    fn symmetric() {
        assert!((scale_at(300.0, 2560.0, 0.12) - scale_at(2260.0, 2560.0, 0.12)).abs() < 1e-9);
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
