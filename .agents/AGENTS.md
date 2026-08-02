# Project Rules for Pasted

## Dual Theme (Light & Dark Mode) Requirement
- ALWAYS ensure all GUI components, buttons, inputs, dropdowns, borders, cards, and text labels support both Dark Mode and Light Mode (`html.light`).
- NEVER hardcode static light/dark color utilities (like `bg-white`, `bg-[#181818]`, `text-gray-300`) without registering a corresponding `html.light` tokenized override class in `src/App.css`.
- Action buttons must use dark background / white text in Dark Mode, and native Apple system colors (or high-contrast light tokens) in Light Mode (`html.light`).

## Tauri 2.0 Window Dragging Permission Requirement
- ALWAYS include `"core:window:allow-start-dragging"` under `permissions` in `src-tauri/capabilities/default.json`.
- In Tauri 2.0, missing `"core:window:allow-start-dragging"` allows unfocused window dragging via macOS Window Server, but silently blocks IPC window dragging when focused.

