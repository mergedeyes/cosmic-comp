// SPDX-License-Identifier: GPL-3.0-only

//! Funnel scale field: windows keep full size in the middle third of an
//! output and shrink uniformly (aspect ratio preserved) towards the edges.
//!
//! Shape (per output, left to right):  \\\\  ==  ==  ////

/// Scale a window has when its center sits directly on the output edge.
pub const MIN_SCALE: f64 = 0.12;

/// Below this scale a window is considered close enough to the edge to dock.
/// Not used yet.
pub const DOCK_THRESHOLD: f64 = 0.15;

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
    fn symmetric() {
        assert!((scale_at(300.0, 2560.0) - scale_at(2260.0, 2560.0)).abs() < 1e-9);
    }
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
