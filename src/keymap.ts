/**
 * 小鹤双拼 · 键位图（菜单栏图标悬停时弹出的提示窗口）
 *
 * 这个窗口不接收鼠标事件、也不抢键盘焦点，纯展示。
 * 映射表来自 Rime double_pinyin_flypy 的 speller/algebra：
 *   zh→v  ch→i  sh→u，ue/üe→t，ong/iong→s，ing/uai→k，iang/uang→l …
 */

interface KeyCap {
  letter: string;
  /** 只有 zh / ch / sh 三个键把声母挪到了别的字母上 */
  initial?: string;
  finals: string[];
}

interface KeyRow {
  /** 键盘排布：下排整体右移，做出错位感 */
  offset: number;
  keys: KeyCap[];
}

const ROWS: KeyRow[] = [
  {
    offset: 0,
    keys: [
      { letter: "Q", finals: ["iu"] },
      { letter: "W", finals: ["ei"] },
      { letter: "E", finals: ["e"] },
      { letter: "R", finals: ["uan"] },
      { letter: "T", finals: ["üe"] },
      { letter: "Y", finals: ["un"] },
      { letter: "U", initial: "sh", finals: ["u"] },
      { letter: "I", initial: "ch", finals: ["i"] },
      { letter: "O", finals: ["o", "uo"] },
      { letter: "P", finals: ["ie"] },
    ],
  },
  {
    offset: 21,
    keys: [
      { letter: "A", finals: ["a"] },
      { letter: "S", finals: ["ong", "iong"] },
      { letter: "D", finals: ["ai"] },
      { letter: "F", finals: ["en"] },
      { letter: "G", finals: ["eng"] },
      { letter: "H", finals: ["ang"] },
      { letter: "J", finals: ["an"] },
      { letter: "K", finals: ["ing", "uai"] },
      { letter: "L", finals: ["iang", "uang"] },
    ],
  },
  {
    offset: 63,
    keys: [
      { letter: "Z", finals: ["ou"] },
      { letter: "X", finals: ["ia", "ua"] },
      { letter: "C", finals: ["ao"] },
      { letter: "V", initial: "zh", finals: ["ui", "ü"] },
      { letter: "B", finals: ["in"] },
      { letter: "N", finals: ["iao"] },
      { letter: "M", finals: ["ian"] },
    ],
  },
];

function buildKey(key: KeyCap): HTMLDivElement {
  const el = document.createElement("div");
  el.className = "km-key";

  const head = document.createElement("div");
  head.className = "km-key-head";

  const letter = document.createElement("span");
  letter.className = "km-letter";
  letter.textContent = key.letter;
  head.appendChild(letter);

  if (key.initial) {
    const badge = document.createElement("span");
    badge.className = "km-initial";
    badge.textContent = key.initial;
    head.appendChild(badge);
  }

  const finals = document.createElement("div");
  finals.className = "km-finals";
  for (const final of key.finals) {
    const span = document.createElement("span");
    span.textContent = final;
    finals.appendChild(span);
  }

  el.appendChild(head);
  el.appendChild(finals);
  return el;
}

function initKeymap(): void {
  const app = document.getElementById("app")!;

  const card = document.createElement("div");
  card.className = "km";

  const head = document.createElement("div");
  head.className = "km-head";
  head.innerHTML = `
    <span class="km-title">小鹤双拼</span>
    <span class="km-hint">移开鼠标即收起</span>
  `;

  const board = document.createElement("div");
  board.className = "km-board";
  for (const row of ROWS) {
    const rowEl = document.createElement("div");
    rowEl.className = "km-row";
    rowEl.style.paddingLeft = `${row.offset}px`;
    for (const key of row.keys) {
      rowEl.appendChild(buildKey(key));
    }
    board.appendChild(rowEl);
  }

  const legend = document.createElement("div");
  legend.className = "km-legend";
  legend.innerHTML = `
    <span>声母　zh → v　ch → i　sh → u</span>
    <span>零声母　首字母 + 韵母键（安 aj）</span>
  `;

  card.append(head, board, legend);
  app.appendChild(card);
}

initKeymap();

export {};
