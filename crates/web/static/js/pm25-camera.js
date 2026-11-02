// The in-page camera of the photo button: a full-screen viewfinder with a guide box, the live
// time in the corner that will be burnt into the photo, pinch-to-zoom, a shutter, a torch switch
// and a close button. Used only where the browser reports torch support; any start error
// rejects, and the caller then offers the phone's own camera app instead.
import { pinchZoom } from "./pm25-camera-zoom.js";
import { stamp } from "./pm25-photo.js";

function element(tag, className, attributes = {}) {
  const node = document.createElement(tag);
  node.className = className;
  for (const [name, value] of Object.entries(attributes)) {
    node.setAttribute(name, value);
  }
  return node;
}

/**
 * Opens the camera and resolves with `{ blob, takenAt }` at the shutter, or `null` when closed.
 * `labels` holds the texts of the three buttons and `hint` the line above the guide box.
 */
export async function capture({ timeZone, labels, hint }) {
  const stream = await navigator.mediaDevices.getUserMedia({
    video: {
      facingMode: "environment",
      width: { ideal: 4096 },
      height: { ideal: 2160 },
    },
    audio: false,
  });
  const track = stream.getVideoTracks()[0];
  const opener = document.activeElement;

  const overlay = element("div", "camera", { role: "dialog", "aria-modal": "true" });
  const video = element("video", "camera-video", { playsinline: "", muted: "", autoplay: "" });
  video.muted = true;
  video.srcObject = stream;
  const view = element("div", "camera-view");
  view.append(video);
  const clock = element("p", "camera-clock");
  const guide = element("div", "camera-guide");
  const level = element("p", "camera-zoom", { "aria-live": "polite" });
  level.hidden = true;
  const note = element("p", "camera-hint");
  note.textContent = hint;
  const bar = element("div", "camera-bar");
  const close = element("button", "camera-button", { type: "button", "aria-label": labels.close });
  close.textContent = "✕";
  const shutter = element("button", "camera-shutter", {
    type: "button",
    "aria-label": labels.shutter,
  });
  const torch = element("button", "camera-button", {
    type: "button",
    "aria-label": labels.torch,
    "aria-pressed": "false",
  });
  torch.textContent = "⚡";
  torch.hidden = !track.getCapabilities?.().torch;
  bar.append(close, shutter, torch);
  overlay.append(view, guide, clock, level, note, bar);
  document.body.append(overlay);
  document.body.classList.add("camera-open");

  const tick = () => {
    clock.textContent = stamp(new Date(), timeZone);
  };
  tick();
  const timer = setInterval(tick, 1000);

  const zoom = pinchZoom({
    surface: overlay,
    view,
    video,
    track,
    onChange: (times) => {
      level.textContent = `${times.toFixed(1)}×`;
      level.hidden = times <= 1;
    },
  });

  return new Promise((resolve, reject) => {
    let finished = false;
    const finish = (outcome, failure) => {
      if (finished) {
        return;
      }
      finished = true;
      clearInterval(timer);
      zoom.stop();
      for (const each of stream.getTracks()) {
        each.stop();
      }
      overlay.remove();
      document.body.classList.remove("camera-open");
      document.removeEventListener("keydown", onKey);
      opener?.focus?.();
      if (failure) {
        reject(failure);
      } else {
        resolve(outcome);
      }
    };
    const onKey = (event) => {
      if (event.key === "Escape") {
        finish(null);
      }
    };
    document.addEventListener("keydown", onKey);
    close.addEventListener("click", () => finish(null));
    track.addEventListener("ended", () => finish(null, new Error("camera stopped")));

    torch.addEventListener("click", async () => {
      const on = torch.getAttribute("aria-pressed") !== "true";
      try {
        await track.applyConstraints({ advanced: [{ torch: on }] });
        torch.setAttribute("aria-pressed", String(on));
      } catch {
        torch.hidden = true;
      }
    });

    shutter.addEventListener("click", () => {
      const takenAt = new Date();
      if (!video.videoWidth) {
        return;
      }
      const part = zoom.crop({ width: video.videoWidth, height: video.videoHeight });
      const canvas = document.createElement("canvas");
      canvas.width = Math.round(part.width);
      canvas.height = Math.round(part.height);
      canvas
        .getContext("2d")
        .drawImage(
          video,
          part.x,
          part.y,
          part.width,
          part.height,
          0,
          0,
          canvas.width,
          canvas.height,
        );
      canvas.toBlob(
        (blob) => (blob ? finish({ blob, takenAt }) : finish(null, new Error("no frame"))),
        "image/jpeg",
        0.95,
      );
    });

    video.play().then(
      () => shutter.focus(),
      (error) => finish(null, error),
    );
  });
}
