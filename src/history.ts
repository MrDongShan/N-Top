import { invoke } from "@tauri-apps/api/core";

interface HistoryEntry {
  filename: string;
  timestamp: string;
  text: string;
}

async function initHistory(): Promise<void> {
  const app = document.getElementById("app")!;

  app.innerHTML = `
    <div class="hist-container">
      <div class="hist-header">
        <span class="hist-title">今天的历史记录</span>
        <button id="btn-refresh" class="hist-refresh">↻ 刷新</button>
      </div>
      <div id="hist-list" class="hist-list">
        <div class="hist-loading">加载中…</div>
      </div>
    </div>
  `;

  const histList = document.getElementById("hist-list") as HTMLDivElement;
  const btnRefresh = document.getElementById("btn-refresh") as HTMLButtonElement;

  async function loadHistory() {
    histList.innerHTML = '<div class="hist-loading">加载中…</div>';
    try {
      const entries = await invoke<HistoryEntry[]>("get_history");
      if (entries.length === 0) {
        histList.innerHTML = '<div class="hist-empty">今天还没有历史记录</div>';
        return;
      }

      histList.innerHTML = entries
        .map((e, i) => {
          const time = e.timestamp.split(" ").pop() || e.timestamp;
          const preview =
            e.text.length > 80 ? e.text.slice(0, 80) + "…" : e.text || "(空)";
          return `
            <div class="hist-item" data-index="${i}">
              <div class="hist-item-head">
                <span class="hist-time">${time}</span>
                <div class="hist-actions">
                  <button class="hist-expand" data-index="${i}">展开</button>
                  <button class="hist-copy" data-index="${i}">复制全部</button>
                </div>
              </div>
              <div class="hist-preview" data-index="${i}">${escapeHtml(preview)}</div>
              <div class="hist-full" data-index="${i}" style="display:none;">${escapeHtml(e.text)}</div>
            </div>`;
        })
        .join("");

      const fullTexts = entries.map((e) => e.text);

      // Toggle expand/collapse
      histList.querySelectorAll(".hist-expand").forEach((btn) => {
        btn.addEventListener("click", (e) => {
          e.stopPropagation();
          const idx = (e.target as HTMLButtonElement).dataset.index || "0";
          const preview = histList.querySelector(`.hist-preview[data-index="${idx}"]`) as HTMLDivElement;
          const full = histList.querySelector(`.hist-full[data-index="${idx}"]`) as HTMLDivElement;
          const btnEl = e.target as HTMLButtonElement;

          if (full.style.display === "none") {
            full.style.display = "block";
            preview.style.display = "none";
            btnEl.textContent = "收起";
          } else {
            full.style.display = "none";
            preview.style.display = "block";
            btnEl.textContent = "展开";
          }
        });
      });

      // Copy full text
      histList.querySelectorAll(".hist-copy").forEach((btn) => {
        btn.addEventListener("click", (e) => {
          e.stopPropagation();
          const idx = parseInt((e.target as HTMLButtonElement).dataset.index || "0");
          navigator.clipboard.writeText(fullTexts[idx]);
          const btnEl = e.target as HTMLButtonElement;
          btnEl.textContent = "已复制";
          setTimeout(() => { btnEl.textContent = "复制全部"; }, 1500);
        });
      });

      // Click item to expand too
      histList.querySelectorAll(".hist-item").forEach((item) => {
        item.addEventListener("click", (e) => {
          if ((e.target as HTMLElement).tagName === "BUTTON") return;
          const idx = (item as HTMLElement).dataset.index || "0";
          const expandBtn = histList.querySelector(`.hist-expand[data-index="${idx}"]`) as HTMLButtonElement;
          expandBtn.click();
        });
      });
    } catch {
      histList.innerHTML = '<div class="hist-empty">加载失败</div>';
    }
  }

  btnRefresh.addEventListener("click", loadHistory);
  loadHistory();

  function escapeHtml(str: string): string {
    return str
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }
}

initHistory();
