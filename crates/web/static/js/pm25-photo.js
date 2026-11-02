// The photo controls of the Log screen (spec §4, §5): take or pick a photo of the sensor
// display, burn the time into it, shrink it until it fits the upload limit, preview it and let
// the server read the number. Every text shown comes from the page; without this script the
// photo button stays hidden and the form posts as before.

export const SIDES = [2400, 2000, 1600, 1280];
export const QUALITIES = [0.85, 0.75, 0.65, 0.55];

/** `dd.mm.yyyy HH:MM` in the given time zone. */
export function stamp(date, timeZone) {
  const parts = Object.fromEntries(
    new Intl.DateTimeFormat("en-GB", {
      timeZone,
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      hourCycle: "h23",
    })
      .formatToParts(date)
      .map((part) => [part.type, part.value]),
  );
  return `${parts.day}.${parts.month}.${parts.year} ${parts.hour}:${parts.minute}`;
}

function burn(context, width, height, text) {
  const size = Math.round(Math.min(width, height) * 0.032);
  const pad = Math.round(size * 0.5);
  context.font = `${size}px sans-serif`;
  context.textBaseline = "top";
  const box = Math.ceil(context.measureText(text).width) + pad * 2;
  context.fillStyle = "rgb(0 0 0 / 70%)";
  context.fillRect(0, 0, box, size + pad * 2);
  context.fillStyle = "#fff";
  context.fillText(text, pad, pad);
}

function encode(canvas, quality) {
  return new Promise((resolve) => canvas.toBlob(resolve, "image/jpeg", quality));
}

/** Draws the photo at `width` x `height`, with the time burnt in when `text` is given. */
function render(canvas, bitmap, width, height, text) {
  const context = canvas.getContext("2d");
  context.imageSmoothingQuality = "high";
  canvas.width = width;
  canvas.height = height;
  context.drawImage(bitmap, 0, 0, width, height);
  if (text) {
    burn(context, width, height, text);
  }
}

/**
 * Draws the photo upright with the time burnt into its top-left corner and encodes it as JPEG:
 * quality 0.85, 0.75, 0.65, 0.55 at a longest side of 2400 px, then the same at 2000, 1600 and
 * 1280 px; the first result within `maxBytes` wins. When none fits, the smallest comes back
 * with `fits: false`.
 *
 * With `plain: true` a winning result also carries `plain`: the same picture at the same size
 * without the time, for the reader only, at the first quality from the winning one downwards
 * that fits (`null` when none does).
 */
export async function shrinkToLimit(source, { maxBytes, takenAt, timeZone, plain = false }) {
  const bitmap = await createImageBitmap(source, { imageOrientation: "from-image" });
  const text = stamp(takenAt, timeZone);
  const canvas = document.createElement("canvas");
  let last = null;
  let lastSize = "";
  try {
    for (const side of SIDES) {
      const scale = Math.min(1, side / Math.max(bitmap.width, bitmap.height));
      const width = Math.round(bitmap.width * scale);
      const height = Math.round(bitmap.height * scale);
      if (`${width}x${height}` === lastSize) {
        continue;
      }
      lastSize = `${width}x${height}`;
      render(canvas, bitmap, width, height, text);
      for (const quality of QUALITIES) {
        const blob = await encode(canvas, quality);
        last = { blob, width, height, quality, fits: blob.size <= maxBytes };
        if (last.fits) {
          if (plain) {
            last.plain = await encodePlain(canvas, bitmap, last, maxBytes);
          }
          return last;
        }
      }
    }
  } finally {
    bitmap.close();
  }
  return last;
}

async function encodePlain(canvas, bitmap, { width, height, quality }, maxBytes) {
  render(canvas, bitmap, width, height, null);
  for (const step of QUALITIES.filter((value) => value <= quality)) {
    const blob = await encode(canvas, step);
    if (blob.size <= maxBytes) {
      return blob;
    }
  }
  return null;
}

const REFERENCES = ["photo_sha256", "photo_bytes", "photo_taken_at"];

function setup(root) {
  const form = root.closest("form");
  const field = form.querySelector("input[name=pm25]");
  const button = root.querySelector("[data-pm25-photo]");
  const file = root.querySelector("input[name=photo]");
  const phone = root.querySelector("[data-photo-use-phone-camera]");
  const hint = root.querySelector("[data-photo-camera-hint]");
  const preview = root.querySelector("[data-photo-preview]");
  const image = root.querySelector("[data-photo-preview-image]");
  const readOk = root.querySelector("[data-photo-read-ok]");
  const invalid = root.querySelector("[data-photo-invalid]");
  const readNone = root.querySelector("[data-photo-read-none]");
  const attached = root.querySelector("[data-photo-attached]");
  const remove = root.querySelector("[data-photo-remove]");
  const timeZone = button.dataset.tz;
  // One photo per log: the take button shows only while none is attached.
  button.hidden = !attached.hidden;
  let working = Promise.resolve();
  let busy = false;
  let generation = 0;
  let previewUrl = null;

  const reference = (name) => form.querySelector(`input[name=${name}]`);

  function setTakenAt(date) {
    for (const name of ["photo_sha256", "photo_bytes"]) {
      reference(name)?.remove();
    }
    let input = reference("photo_taken_at");
    if (!input) {
      input = document.createElement("input");
      input.type = "hidden";
      input.name = "photo_taken_at";
      form.prepend(input);
    }
    input.value = date.toISOString();
  }

  function clearRead() {
    readOk.hidden = true;
    readNone.hidden = true;
    invalid.hidden = true;
  }

  function dropPreview() {
    if (previewUrl) {
      URL.revokeObjectURL(previewUrl);
      previewUrl = null;
    }
    image.removeAttribute("src");
    preview.hidden = true;
  }

  function clearFromPhoto() {
    delete field.dataset.fromPhoto;
  }

  async function read(blob, mine) {
    if (!blob) {
      if (mine === generation) {
        readNone.hidden = false;
      }
      return;
    }
    const body = new FormData();
    body.append("photo", blob, "pm25.jpg");
    let value = null;
    try {
      const response = await fetch("/log/pm25/read", { method: "POST", body });
      if (response.ok) {
        value = (await response.json()).value;
      }
    } catch {
      value = null;
    }
    if (mine !== generation) {
      return;
    }
    if (typeof value !== "number") {
      readNone.hidden = false;
    } else if (field.value === "" || "fromPhoto" in field.dataset) {
      field.value = String(value);
      field.dataset.fromPhoto = "";
      readOk.hidden = false;
    }
  }

  async function attach(source, takenAt) {
    const mine = ++generation;
    clearRead();
    const shrunk = await shrinkToLimit(source, {
      maxBytes: Number(button.dataset.maxBytes),
      takenAt,
      timeZone,
      plain: true,
    });
    if (mine !== generation) {
      return;
    }
    if (!shrunk.fits) {
      throw new Error("photo too large");
    }
    const upload = new File([shrunk.blob], "pm25.jpg", { type: "image/jpeg" });
    const transfer = new DataTransfer();
    transfer.items.add(upload);
    file.files = transfer.files;
    setTakenAt(takenAt);
    if (previewUrl) {
      URL.revokeObjectURL(previewUrl);
    }
    previewUrl = URL.createObjectURL(upload);
    image.src = previewUrl;
    preview.hidden = false;
    attached.hidden = false;
    remove.hidden = false;
    button.hidden = true;
    phone.hidden = true;
    // The reader gets the picture without the burnt-in time: its digits are not the sensor's.
    await read(shrunk.plain, mine);
  }

  function run(source, takenAt) {
    busy = true;
    button.hidden = true;
    working = attach(source, takenAt)
      .catch(() => {
        generation += 1;
        file.value = "";
        clearRead();
        invalid.hidden = false;
        button.hidden = !attached.hidden;
      })
      .finally(() => {
        busy = false;
      });
    return working;
  }

  function discard() {
    generation += 1;
    for (const name of REFERENCES) {
      reference(name)?.remove();
    }
    file.value = "";
    dropPreview();
    clearRead();
    // A number read from the photo goes with it, so the next photo can fill the field again.
    if ("fromPhoto" in field.dataset) {
      field.value = "";
    }
    clearFromPhoto();
    attached.hidden = true;
    remove.hidden = true;
    button.hidden = false;
  }

  async function take() {
    phone.hidden = true;
    if (!navigator.mediaDevices?.getSupportedConstraints?.().torch) {
      file.click();
      return;
    }
    try {
      const { capture } = await import("./pm25-camera.js");
      const shot = await capture({
        timeZone,
        hint: hint?.textContent ?? "",
        labels: {
          close: button.dataset.labelClose,
          shutter: button.dataset.labelShutter,
          torch: button.dataset.labelTorch,
        },
      });
      if (shot) {
        await run(shot.blob, shot.takenAt);
      }
    } catch {
      phone.hidden = false;
    }
  }

  button.addEventListener("click", take);
  phone.addEventListener("click", () => {
    phone.hidden = true;
    file.click();
  });
  file.addEventListener("change", () => {
    const chosen = file.files[0];
    if (chosen) {
      const taken = chosen.lastModified ? new Date(chosen.lastModified) : new Date();
      run(chosen, taken);
    }
  });
  remove.addEventListener("click", discard);
  field.addEventListener("input", (event) => {
    if (event.isTrusted) {
      clearFromPhoto();
      readOk.hidden = true;
    }
  });

  let sending = false;
  form.addEventListener("submit", (event) => {
    if (sending) {
      event.preventDefault();
      return;
    }
    if (busy) {
      event.preventDefault();
      const submitter = event.submitter;
      const seen = field.value;
      working.then(() => {
        if (field.value !== seen) {
          field.focus();
          return;
        }
        form.requestSubmit(submitter);
      });
      return;
    }
    sending = true;
    // The form data is built right after this event; disabling at once would drop the
    // submitter's own name and value.
    setTimeout(() => {
      for (const control of form.querySelectorAll("button[type=submit]")) {
        control.disabled = true;
      }
    }, 0);
  });
  addEventListener("pageshow", (event) => {
    if (event.persisted) {
      sending = false;
      for (const control of form.querySelectorAll("button[type=submit]")) {
        control.disabled = false;
      }
    }
  });
}

const root = document.querySelector("[data-photo]");
if (root) {
  setup(root);
}
