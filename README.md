<p align="center">
  <img src="assets/logo.svg" width="96" height="96" alt="schet logo">
</p>

# schet (`счёт`)

**A native Rust + GTK3 notepad calculator built because feeding 700+ MB of RAM to an entire Chromium instance just to split a pizza bill or figure out taxes is ridiculous.**

`schet` takes the best ideas from [Numara Calculator](https://github.com/bornova/numara-calculator) and Soulver, then throws Electron, Chromium, Node.js, WebKitGTK, and two thousand `node_modules` packages out the window. What remains is a single **472 KB** native binary, cold start faster than a single monitor frame, and **12.6 MB** of actual private memory.

![schet — Dark theme](assets/dark.png)
![schet — Light theme](assets/light.png)

---

## Why I built this (in numbers, not marketing)

I initially tried the lazy route: ripping Electron out of Numara and wrapping its `mathjs` + `CodeMirror` frontend in Tauri / Wry on top of the system `WebKitGTK`. Spoiler: a browser engine is still a browser engine even if you slap a "Built with Rust" sticker on the box.

Here are the real memory measurements from `/proc/<pid>/smaps_rollup` on the exact same calculation sheet (Arch Linux / Wayland / Niri):

| Build | Stack | Processes | Binary Size | Total RSS | PSS | Private Dirty (actual footprint) |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **Numara (`numara-bin`)** | Electron 45 + Chromium + V8 | 6 | ~180 MB | 738.2 MB | 378.9 MB | **178.6 MB** |
| **Wry / Tauri prototype** | Rust + WebKitGTK 4.1 + JavaScriptCore | 4 | 3.5 MB | 509.9 MB | 361.8 MB | **225.7 MB** |
| **`schet`** | **Pure Rust + GTK3 (native UI & parser)** | **1** | **472 KB** | **73.6 MB** | **34.0 MB** | **12.6 MB** |

Zero background workers, zero network calls, zero telemetry, zero auto-updaters, and no bulky titlebars. Open it, type your math, click an answer to copy, move on.

---

## What the engine understands

`schet` runs a custom recursive-descent parser (`src/eval.rs`) with zero external math crates. It evaluates messy human notes mixed with formulas on every keystroke:

- **Variables, labels, and compound assignments (English & Cyrillic):**
  ```text
  income = 250 000 + 65 000       -> 315 000
  tax = 6% of income              -> 18 900
  rent: 65 000                    -> 65 000
  coffee: 12 500 + 4 300          -> 16 800
  income += 50k                   -> 365 000
  ```
- **Copy-paste friendly numbers & decimal commas:**
  Copied `315 000` from the answer rail or a bank statement and pasted it back into an expression? `schet` doesn't choke on space-grouped thousands (`250 000 + 65 000`), understands magnitude suffixes (`15k`, `250к`, `2.5m`, `1.2м`, `1b`), hex (`0xff`), binary (`0b1010`), and decimal commas (`3,14 * 2`) — while still telling a decimal comma apart from function argument commas in `round(3,1415926 * 2, 3)` or `pow(2,10)`.
- **Human-style percentages:**
  - `1250 + 20%` → `1 500`
  - `1250 - 20%` → `1 000`
  - `20% of 1250` / `6% от 250к` / `15% из 80 000`
  - `120 / (1 + 20%)` → `100`
- **Line references & block aggregations:**
  - `#1`, `#5`, `line2`, `строка1` — reference the result of any specific line.
  - `prev`, `ans`, `_`, `пред`, `ответ` — grab the previous evaluated line's result.
  - `sum` / `total` / `сумма` / `итого` and `avg` / `mean` / `среднее` — sum or average the current contiguous block of lines (up to an empty line) or explicit arguments.
- **Built-in math functions, degrees & operators:**
  - `sqrt` (`корень`), `cbrt`, `abs` (`модуль`), `round(x, [n])` (`окр`), `floor`, `ceil`, `trunc`
  - `pow(a, b)` (`степень`), `min(...)` (`мин`), `max(...)` (`макс`), `fact(n)` and postfix factorial `5!`
  - `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `ln`, `log`, `log2`, `log10`, `exp`
  - Degree postfixes `deg` / `°` / `град`: `sin(45 deg)^2 + cos(45°)^2` → `1`
  - Integer division `10 // 3` → `3` and modulo `10 % 3` / `10 mod 3` → `1` (smart enough to distinguish `10 // 3` from an inline comment like `250k // salary`).

---

## Interface & workflow

- **Frameless Amber Field UI:** No wasted top titlebar or cluttered toolbar. Just a numbered gutter (`01`, `02`...), your scratchpad, the right-aligned answer rail, and a single hardware-style bottom bar (which also acts as a drag handle if you want to move the window with a mouse).
- **Live system theme sync:** Automatically switches in real time between matte graphite Dark (`#121210`, phosphor amber answers `#F59E0B`, acid-lime hover `#A3E635`) and crisp pure-white Light (`#FFFFFF`, amber-gold answers `#D97706`, emerald hover `#16A34A`).
- **One-click copy:** Hover over any result in the right column to highlight it, click once, and it's in your system clipboard.
- **Multi-sheet persistence:** All sheets and notes are saved automatically to `~/.local/share/schet/state.json` as you type.

### Keyboard shortcuts

| Shortcut | Action |
| :--- | :--- |
| **Left-click any answer** | Copy that line's result to clipboard |
| **`Ctrl + N`** | Create a new sheet |
| **`Ctrl + PageUp` / `Alt + Left`** | Previous sheet |
| **`Ctrl + PageDown` / `Alt + Right`** | Next sheet |
| **`Ctrl + L`** | Clear current sheet (pressing again on an empty extra sheet deletes it) |
| **`Ctrl + Q` / `Ctrl + W`** | Quit |

---

## Build & install

All you need is a standard Rust toolchain (`cargo`) and `gtk3` development headers.

```bash
git clone https://github.com/mqrkin/schet.git
cd schet
cargo test
cargo build --release

# Install binary to ~/.local/bin
install -Dm755 target/release/schet ~/.local/bin/schet
```

## License

[MIT](LICENSE)
