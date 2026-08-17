import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

interface AppSettings {
  text: string;
  pos_x: number;
  pos_y: number;
  bg_alpha: number;
  font_size: number;
  font_color: string;
  hotkey: string;
}

// ---- Router: detect which window we're in ----
const winLabel = getCurrentWebviewWindow().label;

if (winLabel === "control-panel" || location.hash === "#/control-panel") {
  import("./control-panel.ts");
} else {
  initOverlay();
}

// =============================================================
// Overlay window — the always-on-top text display
// =============================================================
async function initOverlay(): Promise<void> {
  const app = document.getElementById("app")!;
  const settings = await invoke<AppSettings>("get_settings");

  const textarea = document.createElement("textarea");
  textarea.id = "overlay-text";
  textarea.value = settings.text;
  textarea.placeholder = "在这里写你的提醒…";
  textarea.spellcheck = false;

  // Apply saved visual settings
  applyVisuals(textarea, settings);

  app.appendChild(textarea);

  // Restore mode
  const mode = await invoke<string>("get_mode");
  applyMode(textarea, mode);

  // Listen for mode changes
  listen<string>("mode-changed", (event) => {
    applyMode(textarea, event.payload);
  });

  // Listen for settings updates from control panel
  listen<AppSettings>("settings-updated", (event) => {
    const s = event.payload;
    applyVisuals(textarea, s);
    // Don't overwrite text while user is typing
    if (document.activeElement !== textarea) {
      textarea.value = s.text;
    }
  });

  // Save text on input (debounced)
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  textarea.addEventListener("input", async () => {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      const current = await invoke<AppSettings>("get_settings");
      current.text = textarea.value;
      await invoke("save_settings_cmd", { settings: current });
    }, 500);
  });

  // Drag window (when in edit mode)
  textarea.addEventListener("mousedown", (e) => {
    if (e.target === textarea && e.detail === 1) {
      // Allow dragging from background, not interfering with text selection
    }
  });

  // Click on empty area to drag
  app.addEventListener("mousedown", async (e) => {
    if (e.target === app || e.target === textarea) {
      // Only drag when clicking outside textarea text area
      if (e.target === app) {
        await getCurrentWebviewWindow().startDragging();
      }
    }
  });

  function applyVisuals(el: HTMLTextAreaElement, s: AppSettings): void {
    el.style.fontSize = `${s.font_size}px`;
    el.style.color = s.font_color;
    document.body.style.backgroundColor = `rgba(0, 0, 0, ${s.bg_alpha})`;
  }

  function applyMode(el: HTMLTextAreaElement, m: string): void {
    if (m === "edit") {
      el.readOnly = false;
      el.style.cursor = "text";
      document.body.classList.remove("pass-through");
      document.body.classList.add("edit-mode");
    } else {
      el.readOnly = true;
      el.style.cursor = "default";
      document.body.classList.remove("edit-mode");
      document.body.classList.add("pass-through");
    }
  }

  // Prevent context menu
  document.addEventListener("contextmenu", (e) => e.preventDefault());
}
