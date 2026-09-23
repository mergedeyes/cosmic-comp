# Funnel

A prototype window-management idea for COSMIC: instead of a fixed set of tiled
or floating windows, the screen itself has *gravity*. Windows shrink smoothly
as they approach the left or right edge, and dock into a stack there once
they're small enough to be more of a reminder than a workspace.

It's inspired by Scott Jenson's talk [*Are we stuck with the same Desktop UX
forever?*](https://www.youtube.com/results?search_query=scott+jenson+desktop+ux+stuck)
(Ubuntu Summit 25.10), and built as a fork of
[`cosmic-comp`](https://github.com/pop-os/cosmic-comp), System76's Wayland
compositor for COSMIC.

## The idea

On a wide monitor, the middle third of the screen behaves exactly like a
normal desktop. Move a window into the outer thirds and it shrinks the
closer it gets to the edge — aspect ratio preserved, nothing warped. The
scale field looks like this, left to right:

```
\\\\   ==   ==   ////
```

- Windows can sit anywhere in the outer thirds and keep whatever scale their
  position gives them — it's a spectrum, not a snap-to-grid.
- Get close enough to an edge and the window docks into a slot there,
  stacking vertically with any other docked windows, out of the way but
  still glanceable and one drag away from being full-size again.
- The scaling is purely visual. The window's client never finds out — its
  buffer size is untouched, only how it's drawn (and where clicks land)
  changes.

The goal isn't a new tiling scheme. It's treating "how much of my attention
is this window getting right now" as something that can vary continuously,
instead of windows being either fully present or minimized out of sight.

## Status

This is an early, personal prototype, not something you'd want as your
daily driver yet. What's implemented so far, on the `funnel` branch:

- **The scale field itself** — the shrink-toward-the-edges math, applied to
  rendering, hit-testing, input mapping, and live dragging.
- **Move-only mode** — once a window is shrunk enough to be impractical to
  use, clicking it anywhere just moves it (with a blue outline) instead of
  passing the click to the app.
- **Docking** — windows dropped very close to an edge snap into a slot
  there and stack with others on the same edge, freeing you from having to
  position them by hand.

Everything here is render-only and reversible — nothing about a window's
actual size or state changes, just how it's drawn and interacted with while
it's off to the side.

Further ideas from Jenson's talk aren't built yet: dragging windows without
raising them, a clipboard/working-memory drawer, an activity timeline, and
a few more — all still at the design stage.

## Trying it

The safest way to try this without touching your real session is to run it
**nested** — as a window inside your existing desktop, so a crash just
closes that one window and nothing else. In short: build normally
(`cargo build`, same as upstream `cosmic-comp`), then launch the resulting
binary with `COSMIC_BACKEND=winit` set, which forces the Wayland-nested
backend instead of taking over the real session. A couple of funnel-specific
environment variables are also worth knowing about:

- `COSMIC_FUNNEL_FLOATING=1` — start workspaces floating rather than tiled,
  without touching your normal autotile setting (useful since a nested
  session shares your real `~/.config/cosmic`).
- `COSMIC_WINIT_FULL_REDRAW=1` — forces a full redraw every frame; harmless,
  sometimes helps with flicker on hybrid-GPU setups.

## Why a fork, not an extension

Shrinking, repositioning, and rerouting *other clients'* windows isn't
something a normal Wayland client can do — it has to happen in the
compositor. There's no runtime plugin API in `cosmic-comp` today (see
[pop-os/cosmic-comp#673](https://github.com/pop-os/cosmic-comp/issues/673)),
so this lives as a fork for now. A more realistic long-term shape is a
small, generic set of hook points in `cosmic-comp` itself — `main.rs`
already just calls `cosmic_comp::run(Default::default())`, and there's a
precedent in `src/hooks.rs` for exactly this kind of extension point — with
the funnel logic implemented against those hooks from a separate crate,
rather than carried as a long-lived patch. Not there yet, but worth keeping
in mind as this grows.

## Credits

Built on top of [`pop-os/cosmic-comp`](https://github.com/pop-os/cosmic-comp),
licensed GPL-3.0-only, same as this fork. The funnel/docking idea and the
rest of the direction come from Scott Jenson's desktop UX talk referenced
above.
