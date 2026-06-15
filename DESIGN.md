# Syncora Design System

## Color Palette

- All colors use **zinc-\*** tokens exclusively.
- No blue, purple, or marketing accent colors anywhere in the UI.
- Status colors (success / warning / error) are scoped to Badge variants only.

## Layout & Scrolling

- **No visible scrollbars** anywhere in the app — sidebar, main content, dialogs, or any other surface.
- Scrollable areas must always carry the `.scrollbar-hidden` class (defined in `index.css`) alongside any `overflow-*` utility.
- Example: `class="overflow-auto scrollbar-hidden"`
- Scrolling via trackpad/keyboard remains fully functional; only the scrollbar chrome is hidden.

## Icons

- Icons must exclusively use **`lucide-solid`**.
- No custom SVGs, emoji icons, or ad-hoc icon libraries.

## Typography

- System font stack via `-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto`.
- Headings: `text-2xl font-bold tracking-tight text-zinc-900`.
- Body: `text-sm text-zinc-700`.
- Muted / secondary text: `text-sm text-zinc-500`.
- Column labels / uppercase labels: `text-xs font-medium text-zinc-500 uppercase tracking-wide`.

## Component Conventions

### Table / List Layouts
- Use CSS Grid (`grid grid-cols-[...]`) for column-based data tables, not `<table>`.
- Add a header row with `bg-zinc-50 border-b border-zinc-200`.
- Rows use `divide-y divide-zinc-100` and `hover:bg-zinc-50/60 transition-colors`.

### Badges
- Variants: `default` (zinc), `success` (emerald), `warning` (amber), `error` (red), `outline`.
- Min-width `5rem`, height `h-5`, fully rounded (`rounded-full`).

### Buttons
- Primary: black fill. Secondary: zinc border. Danger: red tint (icon buttons only).
- **All buttons must carry a fixed `min-w` from the start** (built into `buttonSizes`): sm/md → `min-w-[88px]`, lg → `min-w-[120px]`. Ghost (icon) buttons use `min-w-0`.
- For buttons whose label changes during async operations, set a wider `min-w` (e.g. `min-w-[120px]`) to accommodate the longest label and prevent layout jitter.
- Loading state: background color shifts (e.g. black → zinc-500), spinner + text update; no `animate-pulse`.

### Dialogs
- Use `structured` prop for header/body/footer layout.
- Errors displayed inline below footer, never as toast.
