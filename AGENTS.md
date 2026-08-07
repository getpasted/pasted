# Pasted contributor guidance

## GUI and CLI parity

Pasted aims to make its meaningful clipboard-management capabilities available through both the graphical app and the CLI.

- When adding or changing a user-facing capability, consider its GUI and CLI surfaces together.
- Prefer shared Rust domain services and data contracts so the GUI and CLI execute the same behavior instead of maintaining parallel implementations.
- Keep terminology, defaults, validation, safety limits, ordering, and error semantics consistent across both surfaces.
- CLI commands should support stable, scriptable output. Prefer a consistent structured-output option such as `--json` when results contain multiple fields.
- GUI-only presentation behavior does not require a CLI equivalent. Features that depend on a running app, operating-system integration, or interactive state may use an explicit live-app command path or remain deferred.
- If a meaningful capability ships on only one surface, document the intentional exception or leave a clearly scoped follow-up before treating the work as complete.
- Add parity-focused tests where practical, especially around shared mutations and structured output contracts.

## Theme-safe styling

- Use Tailwind utilities for structure: layout, spacing, sizing, responsive behavior, and typography.
- Use Pasted semantic theme classes and CSS custom properties for colors, surfaces, borders, dividers, focus rings, and visual emphasis.
- Do not introduce Tailwind's default palette utilities directly into user-facing UI. If a reusable semantic class does not exist, add one to the theme primitives instead of special-casing a component.
- Mechanical utilities such as `divide-y` must be paired with the corresponding semantic class, such as `theme-divide`.
- Keep the CSS architecture audit budgets ratcheting downward; never increase a debt budget to accommodate new styling.
