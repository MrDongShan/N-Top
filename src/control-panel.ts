import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

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

function normalizeFlashInterval(seconds: number | undefined): number {
  return Math.max(5, seconds || 300);
}

function formatFlashInterval(seconds: number): string {
  if (seconds < 60) {
    return `${seconds}秒`;
  }

  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return remainingSeconds === 0 ? `${minutes}分钟` : `${minutes}分${remainingSeconds}秒`;
}

async function initControlPanel(): Promise<void> {
  const app = document.getElementById("app")!;
  const settings = await invoke<AppSettings>("get_settings");
  settings.text_flash_interval_seconds = normalizeFlashInterval(settings.text_flash_interval_seconds);
  let autoStartEnabled = await isEnabled().catch(() => false);
  let mode = await invoke<string>("get_mode");

  const toggleLabel = mode === "edit" ? "编辑中 · 点击穿透" : "穿透中 · 点击编辑";
  const toggleIcon = mode === "edit" ? "✏" : "↗";

  app.innerHTML = `
    <div class="cp">
      <div class="cp-header" data-tauri-drag-region>
        <span class="cp-title" data-tauri-drag-region>N-Top</span>
        <button id="btn-close" class="cp-close" title="收起面板（Esc）">✕</button>
      </div>

      <div class="cp-pinned">
        <button id="btn-toggle" class="cp-toggle ${mode === "edit" ? "is-edit" : "is-passthrough"}">
          <span class="cp-toggle-icon">${toggleIcon}</span>
          <span class="cp-toggle-text">${toggleLabel}</span>
        </button>
      </div>

      <div class="cp-body">
        <div class="cp-row-item">
          <div class="cp-row-head">
            <span class="cp-row-title">字号</span>
            <span class="cp-row-value" id="font-val">${settings.font_size}</span>
          </div>
          <input type="range" id="slider-font" min="12" max="48" value="${settings.font_size}" class="cp-range" />
        </div>

        <div class="cp-row-item">
          <div class="cp-row-head">
            <span class="cp-row-title">背景透明度</span>
            <span class="cp-row-value" id="alpha-val">${Math.round(settings.bg_alpha * 100)}<small>%</small></span>
          </div>
          <input type="range" id="slider-alpha" min="0" max="100" value="${Math.round(settings.bg_alpha * 100)}" class="cp-range" />
        </div>

        <div class="cp-row-item">
          <div class="cp-row-head">
            <span class="cp-row-title">文字闪亮间隔</span>
            <span class="cp-row-value" id="flash-interval-val">${formatFlashInterval(settings.text_flash_interval_seconds)}</span>
          </div>
          <input type="range" id="slider-flash-interval" min="5" max="600" step="5" value="${settings.text_flash_interval_seconds}" class="cp-range" />
        </div>

        <div class="cp-row-item cp-inline">
          <div class="cp-inline-label">
            <span class="cp-row-title">字体颜色</span>
            <span class="cp-color-text" id="color-label">${settings.font_color}</span>
          </div>
          <label id="font-swatch" class="cp-swatch" style="--swatch-color: ${settings.font_color}">
            <input type="color" id="picker-color" value="${settings.font_color}" />
          </label>
        </div>

        <div class="cp-row-item cp-inline">
          <div class="cp-inline-label">
            <span class="cp-row-title">背景颜色</span>
            <span class="cp-color-text" id="bg-color-label">${settings.bg_color}</span>
          </div>
          <label id="bg-swatch" class="cp-swatch" style="--swatch-color: ${settings.bg_color}">
            <input type="color" id="picker-bg-color" value="${settings.bg_color}" />
          </label>
        </div>

        <div class="cp-row-item cp-inline">
          <span class="cp-row-title">文字描边</span>
          <label class="cp-switch">
            <input type="checkbox" id="check-stroke" ${settings.text_stroke ? "checked" : ""} />
            <span class="cp-switch-track"></span>
          </label>
        </div>

        <div class="cp-row-item cp-inline">
          <div class="cp-inline-label">
            <span class="cp-row-title">仅显示第一段</span>
            <span class="cp-tip">只显示第一个换行符之前的文本</span>
          </div>
          <label class="cp-switch">
            <input type="checkbox" id="check-first-paragraph" ${settings.show_first_paragraph ? "checked" : ""} />
            <span class="cp-switch-track"></span>
          </label>
        </div>

        <div class="cp-row-item cp-inline">
          <div class="cp-inline-label">
            <span class="cp-row-title">在程序坞显示</span>
            <span class="cp-tip">关闭则仅在菜单栏显示</span>
          </div>
          <label class="cp-switch">
            <input type="checkbox" id="check-dock" ${settings.show_in_dock ? "checked" : ""} />
            <span class="cp-switch-track"></span>
          </label>
        </div>

        <div class="cp-row-item cp-inline">
          <div class="cp-inline-label">
            <span class="cp-row-title">开机启动</span>
            <span class="cp-tip" id="autostart-tip">${autoStartEnabled ? "登录 macOS 后自动启动 N-Top" : "关闭则不会随系统启动"}</span>
          </div>
          <label class="cp-switch">
            <input type="checkbox" id="check-autostart" ${autoStartEnabled ? "checked" : ""} />
            <span class="cp-switch-track"></span>
          </label>
        </div>

        <div class="cp-row-item">
          <div class="cp-row-head">
            <span class="cp-row-title">全局热键</span>
          </div>
          <input type="text" id="input-hotkey" value="${settings.hotkey}" class="cp-hotkey-input" placeholder="按下组合键…" readonly />
          <span class="cp-tip">点击后按下组合键 · Esc 取消</span>
        </div>

      </div>

      <div class="cp-footer">
        <div class="cp-footer-row">
          <button id="btn-history" class="cp-history-btn">历史记录</button>
          <button id="btn-quit" class="cp-quit">退出 N-Top</button>
        </div>

        <p class="cp-foot">编辑态 ⌘ + 拖动 = 移动窗口</p>
      </div>
    </div>
  `;

  const btnToggle = document.getElementById("btn-toggle") as HTMLButtonElement;
  const toggleIconEl = btnToggle.querySelector(".cp-toggle-icon") as HTMLSpanElement;
  const toggleTextEl = btnToggle.querySelector(".cp-toggle-text") as HTMLSpanElement;
  const sliderAlpha = document.getElementById("slider-alpha") as HTMLInputElement;
  const alphaVal = document.getElementById("alpha-val") as HTMLSpanElement;
  const sliderFont = document.getElementById("slider-font") as HTMLInputElement;
  const fontVal = document.getElementById("font-val") as HTMLSpanElement;
  const sliderFlashInterval = document.getElementById("slider-flash-interval") as HTMLInputElement;
  const flashIntervalVal = document.getElementById("flash-interval-val") as HTMLSpanElement;
  const pickerColor = document.getElementById("picker-color") as HTMLInputElement;
  const colorLabel = document.getElementById("color-label") as HTMLSpanElement;
  const swatchLabel = document.getElementById("font-swatch") as HTMLLabelElement;
  const pickerBgColor = document.getElementById("picker-bg-color") as HTMLInputElement;
  const bgColorLabel = document.getElementById("bg-color-label") as HTMLSpanElement;
  const bgSwatchLabel = document.getElementById("bg-swatch") as HTMLLabelElement;
  const checkStroke = document.getElementById("check-stroke") as HTMLInputElement;
  const checkFirstParagraph = document.getElementById("check-first-paragraph") as HTMLInputElement;
  const inputHotkey = document.getElementById("input-hotkey") as HTMLInputElement;
  const btnHistory = document.getElementById("btn-history") as HTMLButtonElement;

  function updateToggle() {
    const isEdit = mode === "edit";
    btnToggle.className = `cp-toggle ${isEdit ? "is-edit" : "is-passthrough"}`;
    toggleIconEl.textContent = isEdit ? "✏" : "↗";
    toggleTextEl.textContent = isEdit ? "编辑中 · 点击穿透" : "穿透中 · 点击编辑";
  }

  btnToggle.addEventListener("click", async () => {
    await invoke("toggle_overlay_mode");
    // 以后端状态为准。后端在切换时会 emit "mode-changed"，
    // 若这里再本地翻转一次会与事件重复，导致按钮文案慢一拍。
    mode = await invoke<string>("get_mode");
    updateToggle();
  });

  listen<string>("mode-changed", (event) => {
    mode = event.payload;
    updateToggle();
  });

  sliderAlpha.addEventListener("input", async () => {
    const val = parseInt(sliderAlpha.value);
    alphaVal.innerHTML = `${val}<small>%</small>`;
    settings.bg_alpha = val / 100;
    await invoke("save_settings_cmd", { settings });
  });

  sliderFont.addEventListener("input", async () => {
    const val = parseInt(sliderFont.value);
    fontVal.textContent = `${val}`;
    settings.font_size = val;
    await invoke("save_settings_cmd", { settings });
  });

  sliderFlashInterval.addEventListener("input", async () => {
    const val = parseInt(sliderFlashInterval.value);
    flashIntervalVal.textContent = formatFlashInterval(val);
    settings.text_flash_interval_seconds = val;
    await invoke("save_settings_cmd", { settings });
  });

  pickerColor.addEventListener("input", async () => {
    // 系统取色器是原生弹窗，会抢走焦点，先声明正在交互
    keepPanelOpen();
    settings.font_color = pickerColor.value;
    colorLabel.textContent = pickerColor.value;
    if (swatchLabel) swatchLabel.style.setProperty("--swatch-color", pickerColor.value);
    await invoke("save_settings_cmd", { settings });
  });

  pickerBgColor.addEventListener("input", async () => {
    keepPanelOpen();
    settings.bg_color = pickerBgColor.value;
    bgColorLabel.textContent = pickerBgColor.value;
    if (bgSwatchLabel) bgSwatchLabel.style.setProperty("--swatch-color", pickerBgColor.value);
    await invoke("save_settings_cmd", { settings });
  });

  checkStroke.addEventListener("change", async () => {
    settings.text_stroke = checkStroke.checked;
    await invoke("save_settings_cmd", { settings });
  });

  checkFirstParagraph.addEventListener("change", async () => {
    settings.show_first_paragraph = checkFirstParagraph.checked;
    await invoke("save_settings_cmd", { settings });
  });

  const checkDock = document.getElementById("check-dock") as HTMLInputElement;
  const checkAutoStart = document.getElementById("check-autostart") as HTMLInputElement;
  const autoStartTip = document.getElementById("autostart-tip") as HTMLSpanElement;
  checkDock.addEventListener("change", async () => {
    settings.show_in_dock = checkDock.checked;
    await invoke("set_dock_visibility", { visible: checkDock.checked });
  });

  checkAutoStart.addEventListener("change", async () => {
    const requested = checkAutoStart.checked;
    checkAutoStart.disabled = true;

    try {
      if (requested) {
        await enable();
      } else {
        await disable();
      }

      autoStartEnabled = await isEnabled();
      checkAutoStart.checked = autoStartEnabled;
      autoStartTip.textContent = autoStartEnabled
        ? "登录 macOS 后自动启动 N-Top"
        : "关闭则不会随系统启动";
    } catch (error) {
      console.error("设置开机启动失败", error);
      checkAutoStart.checked = autoStartEnabled;
      autoStartTip.textContent = "设置失败，请重试";
    } finally {
      checkAutoStart.disabled = false;
    }
  });

  // Hotkey capture
  let capturing = false;
  inputHotkey.addEventListener("focus", () => {
    capturing = true;
    inputHotkey.classList.add("capturing");
  });

  inputHotkey.addEventListener("blur", () => {
    capturing = false;
    inputHotkey.classList.remove("capturing");
  });

  inputHotkey.addEventListener("keydown", async (e) => {
    if (!capturing) return;
    e.preventDefault();

    if (e.key === "Escape") {
      // 只取消录制，别顺手把面板也收起来
      e.stopPropagation();
      inputHotkey.blur();
      return;
    }

    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Control");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Command");

    if (["Control", "Alt", "Shift", "Meta"].includes(e.key) || parts.length === 0) {
      return;
    }

    const keyName = e.code.replace("Key", "").replace("Digit", "");
    parts.push(keyName);
    const hotkey = parts.join("+");
    inputHotkey.value = hotkey;
    settings.hotkey = hotkey;
    await invoke("save_settings_cmd", { settings });
    inputHotkey.blur();
  });

  function hidePanel(): void {
    void invoke("hide_control_panel_cmd");
  }

  /** 面板正在被操作时，请后端把自动收起再推迟一会儿 */
  function keepPanelOpen(): void {
    void invoke("keep_control_panel_open").catch(() => {});
  }

  document.getElementById("btn-close")?.addEventListener("click", hidePanel);

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      hidePanel();
    }
  });

  swatchLabel?.addEventListener("pointerdown", keepPanelOpen);
  bgSwatchLabel?.addEventListener("pointerdown", keepPanelOpen);

  btnHistory.addEventListener("click", () => {
    // 历史记录是独立窗口，复用同一个实例；切换焦点期间别让面板自动收起
    keepPanelOpen();
    void invoke("open_history_window_cmd").catch((error) => {
      console.error("打开历史记录窗口失败", error);
    });
  });

  document.getElementById("btn-quit")?.addEventListener("click", () => {
    void invoke("quit_app");
  });

  // 无边框窗口里右键会弹出 WebKit 菜单，屏蔽掉
  document.addEventListener("contextmenu", (event) => event.preventDefault());
}

initControlPanel();
