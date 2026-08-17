# Localization

Pasted uses one versioned locale manifest and one message catalog per shipped language. The React interface and Rust-native menus share the catalogs under `src/locales/`.

English (`en`) is the canonical source language and the runtime fallback. A missing translation must never produce an empty control or expose an internal message key.

The stable dotted message keys and variable contracts are the durable interface. The current lightweight formatter can be replaced by Fluent or another standards-based engine later without changing callers or renaming the dictionary. Machine and operating-system translation can generate draft values, but reviewed catalogs remain the deterministic source used for product-authored interface copy.

## What is localized

- GUI navigation, settings, dialogs, controls, validation, notifications, and Help
- Native application and tray menus
- User-facing numbers, dates, times, relative times, plurals, and lists

Stable identifiers and user data are not localized:

- CLI commands, flags, exit behavior, and `--json` field names
- Database values, setting keys, Activity event names, and exported contract keys
- Clipboard contents, transformation input or output, user-defined names, file paths, and credentials
- Literal provider, product, and operating-system names where those names identify an external surface

Shipped registry metadata is a presentation exception to the database rule. Built-in Content Types,
classifiers, extractors, analysis items, and Operations use their stable identifiers to resolve localized
labels and descriptions while their canonical database values remain unchanged. A user-modified built-in
name or description is treated as user data and displayed exactly as saved.

Human-readable CLI output remains English until a dedicated CLI locale contract is introduced. The CLI can still read and set the `language` setting, and it applies the same validation as the GUI.

## Adding a language

1. Copy `src/locales/en.json` to a catalog named with the canonical locale code, such as `es.json` or `pt-BR.json`.
2. Translate values only. Keep dotted keys and `{placeholder}` names unchanged.
3. Add the locale to `src/locales/manifest.json`, including its English name, native name, and text direction.
4. Embed and register the catalog in `src-tauri/src/localization.rs` so native menus use the same messages.
5. Run `npm run test:i18n`, `npm run build`, and the Rust tests.
6. Review destructive actions, privacy and security copy, backup and restore copy, and platform-specific instructions with a fluent reviewer before treating the catalog as shippable.

Catalogs registered in the manifest must be complete. Work-in-progress catalogs may live outside the manifest, but they are not presented in Settings and are not shipped as supported languages.

### Local draft generation

An optional local Ollama workflow can create or resume a draft without sending interface copy to a hosted service:

```sh
node scripts/generate-locale-draft.js --locale=ja-JP --language=Japanese --batch-size=50
```

The generator requires a locally available `translategemma:4b` model, preserves interpolation syntax,
writes progress after every batch, and resumes incomplete work. Use `--prefix=registry.` to limit a run
to one key namespace and `--only-identical` to replace only values still identical to English.

Generated text is a draft, not an approval step. Locale-specific editorial scripts capture reviewed product
terminology and can be rerun after draft regeneration. Always follow generation with `npm run test:i18n`
and review destructive, security, privacy, backup, and platform-specific copy in context.

## Message conventions

- Use stable semantic keys; do not encode the English wording in a key.
- Keep complete sentences and control labels aligned with the product voice rules in `AGENTS.md`.
- Use plural maps with an `other` variant for counted nouns. Add locale-specific CLDR categories such as `few` or `many` when needed.
- Use placeholders for runtime values. Never assemble a translated sentence from separately translated fragments.
- Keep markup out of catalog values. Components own structure and styling.
- Prefer `Intl` formatters through `useLocalization` for user-facing numbers and dates.

## Runtime behavior

`Automatic` follows the operating-system language when a matching catalog is available and otherwise falls back to English. Explicit choices use their selected locale. React updates the root `lang` and `dir` attributes, and language changes rebuild the native application and tray menus.

The browser cache prevents a language flash during startup; SQLite remains authoritative after settings hydrate. Full Backup includes the durable setting through the existing complete-settings snapshot. History and Organization transfer intentionally does not treat interface preferences as portable library content.

### Right-to-left languages

The effective locale sets the document `dir` attribute. Direction-sensitive layout uses logical inline-start and inline-end utilities and CSS properties, while directional navigation and disclosure icons mirror under RTL.

Interface direction does not force clipboard content into the same direction. User content and user-defined labels use automatic direction, technical values such as paths remain explicitly LTR where needed, and values interpolated into RTL messages are wrapped in Unicode bidirectional isolates. Run the RTL audit whenever layout or mixed-content presentation changes.

## Quality gates

`scripts/audit-localization.js` checks:

- manifest structure, unique locale codes, catalog filenames, and text direction;
- complete key coverage for every shipped catalog;
- valid message and plural shapes;
- matching interpolation placeholders;
- shared native-menu catalog registration and usage;
- a ratcheted ceiling for existing hardcoded GUI copy.

The hardcoded-copy ceiling is migration debt. Lower it whenever existing literals move into the catalog; never increase it to accommodate new interface copy.
