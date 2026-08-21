import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

interface AppSettings {
  text: string;
  pos_x: number;
  pos_y: number;
  bg_alpha: number;
  bg_color: string;
  font_size: number;
  font_color: string;
  hotkey: string;
  text_stroke: boolean;
  show_in_dock: boolean;
  text_flash_interval_seconds: number;
  show_first_paragraph: boolean;
}

function getDisplayText(text: string, showFirstParagraph: boolean): string {
  if (!showFirstParagraph) {
    return text;
  }

  const firstLineBreak = text.search(/\r?\n/);
  return firstLineBreak === -1 ? text : text.slice(0, firstLineBreak);
}

function toRgba(color: string, alpha: number): string {
  const hex = color.replace("#", "").trim();
  const normalized = hex.length === 3
    ? hex.split("").map((char) => `${char}${char}`).join("")
    : hex;
  const value = Number.parseInt(normalized, 16);

  if (!Number.isFinite(value) || normalized.length !== 6) {
    return `rgba(255, 250, 240, ${alpha})`;
  }

  const red = (value >> 16) & 255;
  const green = (value >> 8) & 255;
  const blue = value & 255;
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

const winLabel = getCurrentWebviewWindow().label;

if (winLabel === "control-panel" || location.hash === "#/control-panel") {
  import("./control-panel.ts");
} else if (winLabel === "history" || location.hash === "#/history") {
  import("./history.ts");
} else {
  initOverlay();
}

async function initOverlay(): Promise<void> {
  const app = document.getElementById("app")!;
  const settings = await invoke<AppSettings>("get_settings");

  const overlay = document.createElement("div");
  overlay.id = "overlay-text";
  overlay.textContent = getDisplayText(settings.text, settings.show_first_paragraph);
  overlay.spellcheck = false;

  const flashLayer = document.createElement("div");
  flashLayer.id = "overlay-flash";
  flashLayer.setAttribute("aria-hidden", "true");

  applyVisuals(overlay, settings);
  flashLayer.style.fontSize = `${settings.font_size}px`;
  flashLayer.style.color = settings.font_color;
  app.appendChild(overlay);
  app.appendChild(flashLayer);

  let flashIntervalSeconds = normalizeFlashInterval(settings.text_flash_interval_seconds);
  let isFlashing = false;
  let flashTimer: ReturnType<typeof setTimeout> | null = null;
  let currentSettings = settings;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  function normalizeFlashInterval(seconds: number | undefined): number {
    return Math.max(5, seconds || 300);
  }

  function scheduleTextFlash() {
    if (flashTimer) {
      clearTimeout(flashTimer);
    }

    flashTimer = setTimeout(() => {
      if (
        !isFlashing &&
        !document.body.classList.contains("edit-mode") &&
        !document.body.classList.contains("mouse-over")
      ) {
        flashText();
      }
      scheduleTextFlash();
    }, flashIntervalSeconds * 1000);
  }

  function updateFlashInterval(seconds: number | undefined) {
    const nextInterval = normalizeFlashInterval(seconds);
    if (nextInterval === flashIntervalSeconds && flashTimer) {
      return;
    }
    flashIntervalSeconds = nextInterval;
    scheduleTextFlash();
  }

  function renderText() {
    overlay.textContent = getDisplayText(currentSettings.text, currentSettings.show_first_paragraph);
  }

  function saveEditedText() {
    currentSettings.text = overlay.innerText ?? "";
    if (saveTimer) {
      clearTimeout(saveTimer);
    }
    saveTimer = setTimeout(async () => {
      await invoke("save_settings_cmd", { settings: currentSettings });
    }, 500);
  }

  function flashText() {
    const text = overlay.innerText ?? "";
    if (!text) return;
    isFlashing = true;

    flashLayer.innerHTML = "";
    const spans: HTMLSpanElement[] = [];
    for (const char of text) {
      const span = document.createElement("span");
      if (char === "\n") {
        span.innerHTML = "<br>";
      } else {
        span.textContent = char;
      }
      span.className = "flash-char";
      flashLayer.appendChild(span);
      spans.push(span);
    }

    const flashColor = "#ffd28a";
    const flashShadow = "0 0 8px rgba(255, 210, 138, 0.9)";
    const stepDelay = Math.max(18, Math.min(60, Math.floor(1200 / spans.length)));
    flashLayer.style.opacity = "1";

    spans.forEach((span, i) => {
      setTimeout(() => {
        span.style.color = flashColor;
        span.style.textShadow = flashShadow;
        setTimeout(() => {
          span.style.color = "";
          span.style.textShadow = "";
        }, 400);
      }, i * stepDelay);
    });

    const totalDuration = spans.length * stepDelay + 600;
    setTimeout(() => {
      flashLayer.innerHTML = "";
      flashLayer.style.opacity = "0";
      isFlashing = false;
    }, totalDuration);
  }

  scheduleTextFlash();

  const mode = await invoke<string>("get_mode");
  applyMode(mode);

  listen<string>("mode-changed", async (event) => {
    // Keep the full source text when display mode only shows the first paragraph.
    if (event.payload === "pass-through" && document.body.classList.contains("edit-mode")) {
      currentSettings.text = overlay.innerText ?? "";
      if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
      }
      await invoke("save_settings_cmd", { settings: currentSettings });
      renderText();
    }
    applyMode(event.payload);
  });

  listen<AppSettings>("settings-updated", (event) => {
    const s = event.payload;
    const draftText = document.body.classList.contains("edit-mode") ? overlay.innerText ?? "" : s.text;
    currentSettings = { ...s, text: draftText };
    applyVisuals(overlay, s);
    flashLayer.style.fontSize = `${s.font_size}px`;
    flashLayer.style.color = s.font_color;
    updateFlashInterval(s.text_flash_interval_seconds);
    if (document.activeElement !== overlay) {
      renderText();
    }
  });

  overlay.addEventListener("input", saveEditedText);

  // ---- Poll mouse position in pass-through mode ----
  let wasInOverlay = false;
  setInterval(async () => {
    const isEdit = document.body.classList.contains("edit-mode");
    if (isEdit) {
      if (wasInOverlay) {
        wasInOverlay = false;
        document.body.classList.remove("mouse-over");
      }
      return;
    }
    try {
      const inOverlay = await invoke<boolean>("is_mouse_in_overlay");
      if (inOverlay !== wasInOverlay) {
        wasInOverlay = inOverlay;
        if (inOverlay) {
          document.body.classList.add("mouse-over");
          document.body.style.backgroundColor = "transparent";
          overlay.style.opacity = "0";
        } else {
          document.body.classList.remove("mouse-over");
          overlay.style.opacity = "";
          // Restore saved bg alpha
          try {
            const s = await invoke<AppSettings>("get_settings");
            document.body.style.backgroundColor = toRgba(s.bg_color, s.bg_alpha);
          } catch {}
        }
      }
    } catch {
      // ignore
    }
  }, 200);

  // ---- ⌘ key drag in edit mode ----
  let cmdPressed = false;

  document.addEventListener("keydown", (e) => {
    if (e.metaKey && !cmdPressed) {
      cmdPressed = true;
      if (document.body.classList.contains("edit-mode")) {
        overlay.style.cursor = "grab";
      }
    }
  });

  document.addEventListener("keyup", (e) => {
    if (!e.metaKey && cmdPressed) {
      cmdPressed = false;
      if (document.body.classList.contains("edit-mode")) {
        overlay.style.cursor = "text";
      }
    }
  });

  document.addEventListener("mousedown", async (e) => {
    if ((e.metaKey || cmdPressed) && document.body.classList.contains("edit-mode")) {
      e.preventDefault();
      e.stopPropagation();
      overlay.style.cursor = "grabbing";
      await getCurrentWebviewWindow().startDragging();
    }
  });

  document.addEventListener("mouseup", () => {
    if (cmdPressed && document.body.classList.contains("edit-mode")) {
      overlay.style.cursor = "grab";
    }
  });

  function applyVisuals(el: HTMLDivElement, s: AppSettings): void {
    el.style.fontSize = `${s.font_size}px`;
    el.style.color = s.font_color;
    document.body.style.backgroundColor = toRgba(s.bg_color, s.bg_alpha);

    if (s.text_stroke) {
      el.style.textShadow =
        "-1px -1px 2px rgba(255,255,255,0.9)," +
        "1px -1px 2px rgba(255,255,255,0.9)," +
        "-1px 1px 2px rgba(255,255,255,0.9)," +
        "1px 1px 2px rgba(255,255,255,0.9)," +
        "0 0 4px rgba(255,255,255,0.7)," +
        "0 0 8px rgba(255,255,255,0.4)";
    } else {
      el.style.textShadow = "";
    }
  }

  function applyMode(m: string): void {
    if (m === "edit") {
      overlay.textContent = currentSettings.text;
      overlay.contentEditable = "true";
      overlay.style.cursor = "text";
      document.body.classList.remove("pass-through");
      document.body.classList.add("edit-mode");
      document.body.classList.remove("mouse-over");
    } else {
      overlay.contentEditable = "false";
      overlay.style.cursor = "default";
      document.body.classList.remove("edit-mode");
      document.body.classList.add("pass-through");
      window.getSelection()?.removeAllRanges();
    }
  }

  document.addEventListener("contextmenu", (e) => e.preventDefault());
}
