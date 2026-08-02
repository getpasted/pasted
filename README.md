# Pasted 📋✨

> A powerful, privacy-first clipboard manager and text transformation workspace for macOS, built with **Tauri 2.0**, **React**, **TypeScript**, and **Rust**.

---

## ✨ Features

- 💬 **Floating Quick HUD Popover**:
  - Triggerable via customizable global hotkeys (default `⌘+Shift+V`).
  - Native speech-bubble popup with dynamic cursor caret pointing tail.
  - Smart multi-monitor positioning that respects menu bars and macOS Dock insets.
  - Zero-flash Cocoa window positioning.

- ⌨️ **Dvorak & Custom Keyboard Layout Support**:
  - Physical key mapping (`Code::Period`) so the hotkey responds to the logical letter **V** regardless of virtual layout.

- 🗄️ **Smart Clipboard History**:
  - Monitors text, code snippets, image screenshots (with base64 previews), hex color codes, and file paths.
  - Persistent local storage backed by SQLite.
  - Full-text search across clipboard clips.

- ⚡ **Filter Sandbox & Transformation Pipelines**:
  - Build and sequence text processing operations (JSON formatting, Case Conversions, Markdown Stripping, Base64/Hex encoding, Regex Find & Replace).
  - Test transformations live in the Filter Sandbox before applying.

- 👁️ **Native macOS Machine Vision OCR**:
  - In-process, hardware-accelerated image text extraction using Apple's Vision Framework (`VNRecognizeTextRequest`).

- 🥞 **Sequential Stack Paste Queue**:
  - Queue multiple clips to paste one-by-one sequentially via customizable stack hotkeys (`Alt+Super+KeyC` to toggle, `Alt+Super+KeyV` to pop).

- 🎨 **Dual Theme Support (Light & Dark Mode)**:
  - System-adapted dark and light themes with native macOS color tokens.
  - Seamless WCAG 2.1 Level AA/AAA compliant high-contrast color tokens.

- 🔒 **100% Local & Private**:
  - No analytics, telemetry, or remote server dependencies. All data remains on your local machine.

---

## 🛠️ Technology Stack

| Component | Technology |
| :--- | :--- |
| **Framework** | [Tauri 2.0](https://tauri.app/) (Rust + Web view) |
| **Frontend** | [React 19](https://react.dev/), [TypeScript](https://www.typescriptlang.org/) |
| **Styling** | [TailwindCSS](https://tailwindcss.com/) |
| **Database** | [SQLite](https://sqlite.org/) |
| **Icons** | [Lucide React](https://lucide.dev/) |

---

## 🚀 Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or higher)
- [Rust](https://www.rust-lang.org/) (v1.75 or higher)
- macOS (macOS 12+ recommended)

### Installation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/your-username/pasted.git
   cd pasted
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Run in development mode**:
   ```bash
   npm run tauri dev
   ```

4. **Run unit tests**:
   ```bash
   npm test
   ```

5. **Build production application**:
   ```bash
   npm run tauri build
   ```

---

## 🧪 Testing

Run all 14 Rust unit test suites across database operations, text filter engines, OCR memory safety, and shortcut layout parsers:

```bash
npm test
```

---

## ⌨️ Global Hotkeys

| Shortcut | Action |
| :--- | :--- |
| `⌘+Shift+V` | Toggle Quick HUD Popover |
| `Alt+Super+C` | Toggle Sequential Stack Recording |
| `Alt+Super+V` | Pop & Paste Next Stack Clipping |
| `1` - `9` | Instant Paste Clip 1–9 from HUD |
| `Esc` | Close Quick HUD |

---

## 📄 License

Distributed under the [MIT License](LICENSE).
