// Pinch-to-zoom for the in-page camera. Two fingers on the viewfinder change the zoom; the camera
// zooms itself where the browser offers it, otherwise the picture is scaled on screen and the same
// middle part is cut out of the frame at the shutter.

const DIGITAL_MAX = 4;

/** The distance between the two fingers on the screen, or `null` without exactly two. */
function spread(points) {
  if (points.size !== 2) {
    return null;
  }
  const [a, b] = [...points.values()];
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * The part of the camera frame shown in a `view`-sized box (`object-fit: contain`) scaled up by
 * `zoom` around its middle, as `{ x, y, width, height }` in frame pixels.
 */
export function visibleCrop(frame, view, zoom) {
  const fit = Math.min(view.width / frame.width, view.height / frame.height);
  const width = Math.min(frame.width, view.width / (fit * zoom));
  const height = Math.min(frame.height, view.height / (fit * zoom));
  return {
    x: (frame.width - width) / 2,
    y: (frame.height - height) / 2,
    width,
    height,
  };
}

/**
 * Follows pinches on `surface` and zooms `track`, or scales `video` inside its clipping `view`
 * when the camera cannot zoom. The camera's starting view is the widest one offered (an
 * ultra-wide lens only shrinks the display), and `onChange` gets the zoom as a multiple of it.
 * Returns `crop(frame)`, the part of a frame to keep at the shutter, and `stop()`.
 */
export function pinchZoom({ surface, view, video, track, onChange }) {
  const range = track.getCapabilities?.().zoom;
  let optical = Boolean(range && range.max > range.min);
  const base = optical
    ? Math.min(Math.max(track.getSettings?.().zoom ?? range.min, range.min), range.max)
    : 1;
  const step = optical && range.step > 0 ? range.step : 0;
  let zoom = base;

  const points = new Map();
  let start = null;
  let pending = null;
  let applying = false;

  const scaleOnScreen = () => {
    video.style.transform = zoom > 1 ? `scale(${zoom})` : "";
  };

  const fallBackToScreen = async () => {
    optical = false;
    pending = null;
    start = null;
    zoom = 1;
    onChange(zoom);
    // Best effort: leave the lens where it started, so the screen zoom is the only one.
    await track.applyConstraints({ advanced: [{ zoom: base }] }).catch(() => undefined);
  };

  const applyToCamera = async () => {
    if (applying) {
      return;
    }
    applying = true;
    while (pending !== null) {
      const next = pending;
      pending = null;
      try {
        await track.applyConstraints({ advanced: [{ zoom: next }] });
      } catch {
        // The camera said it could zoom but refused: carry on with the on-screen zoom.
        await fallBackToScreen();
      }
    }
    applying = false;
  };

  const set = (wanted) => {
    const [low, high] = optical ? [base, range.max] : [1, DIGITAL_MAX];
    let bounded = Math.min(Math.max(wanted, low), high);
    if (step) {
      bounded = Math.min(base + Math.round((bounded - base) / step) * step, high);
    }
    if (bounded === zoom) {
      return;
    }
    zoom = bounded;
    if (optical) {
      pending = zoom;
      applyToCamera();
    } else {
      scaleOnScreen();
    }
    onChange(optical ? zoom / base : zoom);
  };

  const down = (event) => {
    points.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const distance = spread(points);
    if (distance) {
      start = { distance, zoom };
    }
  };
  const move = (event) => {
    if (!points.has(event.pointerId)) {
      return;
    }
    points.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const distance = spread(points);
    if (distance && start) {
      set(start.zoom * (distance / start.distance));
    }
  };
  const up = (event) => {
    points.delete(event.pointerId);
    start = null;
  };
  // Safari's own page pinch, which `touch-action` alone does not always stop.
  const gesture = (event) => event.preventDefault();

  surface.addEventListener("pointerdown", down);
  surface.addEventListener("pointermove", move);
  for (const name of ["pointerup", "pointercancel"]) {
    surface.addEventListener(name, up);
  }
  surface.addEventListener("gesturestart", gesture);
  surface.addEventListener("gesturechange", gesture);

  return {
    crop(frame) {
      if (optical || zoom === 1) {
        return { x: 0, y: 0, ...frame };
      }
      const box = view.getBoundingClientRect();
      return visibleCrop(frame, { width: box.width, height: box.height }, zoom);
    },
    stop() {
      surface.removeEventListener("pointerdown", down);
      surface.removeEventListener("pointermove", move);
      for (const name of ["pointerup", "pointercancel"]) {
        surface.removeEventListener(name, up);
      }
      surface.removeEventListener("gesturestart", gesture);
      surface.removeEventListener("gesturechange", gesture);
    },
  };
}
