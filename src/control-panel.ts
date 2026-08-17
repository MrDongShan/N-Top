import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface AppSettings {
  text: string;
  pos_x: number;
  pos_y: number;
  bg_alpha: number;
  font_size: number;
  font_color: string;
  hotkey: string;
}

async function initControlPanel(): Promise<void> {
  const app = document.getElementById("app")!;
  const settings = await invoke<AppSettings>("get_settings");
  let mode = await invoke<string>("get_mode");

  app.innerHTML = `
    <div class="cp-container">
      <h2>N-Top 控制面板</h2>

      <div class="cp-section">
        <button id="btn-toggle" class="cp-btn primary">
          ${mode === "edit" ? "✏️ 正在编辑（点击切换穿透）" : "🖱️ 穿透中（点击可编辑）"}
        </button>
      </div>

      <div class="cp-section">
        <label class="cp-label">背景透明度: <span id="alpha-val">${Math.round(settings.bg_alpha * 100)}%</span></label>
        <input type="range" id="slider-alpha" min="0" max="100" value="${Math.round(settings.bg_alpha * 100)}" class="cp-slider" />
      </div>

      <div class="cp-section">
        <label class="cp-label">字号: <span id="font-val">${settings.font_size}px</span></label>
        <input type="range" id="slider-font" min="12" max="48" value="${settings.font_size}" class="cp-slider" />
      </div>

      <div class="cp-section">
        <label class="cp-label">字体颜色</label>
        <div class="cp-row">
          <input type="color" id="picker-color" value="${settings.font_color}" class="cp-color" />
          <span class="cp-color-text">${settings.font_color}</span>
        </div>
      </div>

      <div class="cp-section">
        <label class="cp-label">全局热键</label>
        <div class="cp-row">
          <input type="text" id="input-hotkey" value="${settings.hotkey}" class="cp-input" placeholder="Alt+Command+E" />
          <button id="btn-hotkey" class="cp-btn small">应用</button>
        </div>
        <p class="cp-hint">首次使用需在「系统设置 → 隐私 → 辅助功能」中授权</p>
      </div>
    </div>
  `;

  const btnToggle = document.getElementById("btn-toggle") as HTMLButtonElement;
  const sliderAlpha = document.getElementById("slider-alpha") as HTMLInputElement;
  const alphaVal = document.getElementById("alpha-val") as HTMLSpanElement;
  const sliderFont = document.getElementById("slider-font") as HTMLInputElement;
  const fontVal = document.getElementById("font-val") as HTMLSpanElement;
  const pickerColor = document.getElementById("picker-color") as HTMLInputElement;
  const colorText = document.querySelector(".cp-color-text") as HTMLSpanElement;
  const inputHotkey = document.getElementById("input-hotkey") as HTMLInputElement;
  const btnHotkey = document.getElementById("btn-hotkey") as HTMLButtonElement;

  // ---- Toggle mode ----
  btnToggle.addEventListener("click", async () => {
    await invoke("toggle_overlay_mode");
    mode = mode === "edit" ? "pass-through" : "edit";
    btnToggle.textContent = mode === "edit"
      ? "✏️ 正在编辑（点击切换穿透）"
      : "🖱️ 穿透中（点击可编辑）";
    btnToggle.classList.toggle("primary", mode === "edit");
  });

  listen<string>("mode-changed", (event) => {
    mode = event.payload;
    btnToggle.textContent = mode === "edit"
      ? "✏️ 正在编辑（点击切换穿透）"
      : "🖱️ 穿透中（点击可编辑）";
    btnToggle.classList.toggle("primary", mode === "edit");
  });

  // ---- Alpha slider ----
  sliderAlpha.addEventListener("input", async () => {
    const val = parseInt(sliderAlpha.value);
    alphaVal.textContent = `${val}%`;
    settings.bg_alpha = val / 100;
    await invoke("save_settings_cmd", { settings });
  });

  // ---- Font size slider ----
  sliderFont.addEventListener("input", async () => {
    const val = parseInt(sliderFont.value);
    fontVal.textContent = `${val}px`;
    settings.font_size = val;
    await invoke("save_settings_cmd", { settings });
  });

  // ---- Color picker ----
  pickerColor.addEventListener("input", async () => {
    settings.font_color = pickerColor.value;
    colorText.textContent = pickerColor.value;
    await invoke("save_settings_cmd", { settings });
  });

  // ---- Hotkey ----
  btnHotkey.addEventListener("click", async () => {
    settings.hotkey = inputHotkey.value.trim();
    await invoke("save_settings_cmd", { settings });
  });
}

initControlPanel();
