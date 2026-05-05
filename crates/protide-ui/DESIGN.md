# Protide Design Language

> Single source of truth for colors, spacing, sizing, and interaction patterns.
> When building UI, match what's here — don't invent new values.

---

## Principles

1. **IDE-density first.** Every pixel earns its place. Use compact rows (32px), not
   generous padding. Copy Zed, not a consumer app.
2. **Dark-native.** The dark palette is the canonical reference. Light is derived.
3. **Semantic tokens, never raw hex.** Always reference `theme.colors.*`, never
   hard-code a color value in component code.
4. **Accent = green (dark) / blue (light).** Accent marks the single active/selected
   state. Don't use accent for decoration.
5. **One elevated surface per stack.** `bg_elevated` is for inputs and floating
   elements only — don't nest elevated surfaces.

---

## Color Tokens

### Backgrounds (dark → light)

| Token | Dark | Light | Use |
|---|---|---|---|
| `bg_primary` | `#0d0d0f` | `#ffffff` | App chrome, main canvas |
| `bg_secondary` | `#111113` | `#f3f3f3` | Sidebar, panels |
| `bg_tertiary` | `#131315` | `#e8e8e8` | Hover targets, inset sections |
| `bg_elevated` | `#1b1b1e` | `#ffffff` | Inputs, dropdowns, tooltips |

Hover overlays are additive on top of any background:
```
hover:  white @ 3.5% opacity  (dark)  /  black @ 4%  (light)
active: white @ 5.5% opacity  (dark)  /  black @ 8%  (light)
```

### Text

| Token | Dark | Light | Use |
|---|---|---|---|
| `text_primary` | `#e4e4ed` | `#1e1e1e` | Body text, labels |
| `text_secondary` | `#7f7f92` | `#616161` | Captions, subtitles, table headers |
| `text_muted` | `#3e3e4a` | `#9e9e9e` | Placeholder, disabled, dividers |

### Borders

| Token | Use |
|---|---|
| `border` | Default divider between surfaces |
| `border_focused` | Input focus ring (= `accent`) |

### Accent

| Token | Dark | Light |
|---|---|---|
| `accent` | `#4ade80` (green) | `#007acc` (blue) |
| `accent_hover` | `#6ee7a0` | `#0066b8` |

Accent tints (use these for backgrounds, not the raw accent):

| Purpose | Expression |
|---|---|
| Badge / pill background | `accent.opacity(0.12)` |
| Active tab background | `accent.opacity(0.15)` |
| Active tab border | `accent` (full opacity) |
| Hover on accent-tinted element | `accent.opacity(0.25)` |
| Inactive badge border | `accent.opacity(0.35)` |

### HTTP Methods

| Method | Color |
|---|---|
| GET | `method_get` — green `#4ade80` |
| POST | `method_post` — blue `#60a5fa` |
| PUT | `method_put` — yellow `#fbbf24` |
| PATCH | `method_patch` — orange `#fb923c` |
| DELETE | `method_delete` — red `#f87171` |
| HEAD | `method_head` — purple `#a78bfa` |
| OPTIONS | `method_options` — slate `#94a3b8` |

Use `theme.method_color(method_str)` — never match manually.
Method chip: `method_color.opacity(0.12)` bg, `method_color` text.

### Protocols

| Protocol | Color |
|---|---|
| WebSocket | `protocol_ws` — emerald `#34d399` |
| gRPC | `protocol_grpc` — indigo `#818cf8` |
| GraphQL | `protocol_graphql` — pink `#f472b6` |

### Status Codes

| Range | Token |
|---|---|
| 2xx | `status_success` (green) |
| 3xx | `status_redirect` (yellow) |
| 4xx | `status_client_error` (orange) |
| 5xx | `status_server_error` (red) |

Use `theme.status_color(status_code)` — never match manually.

### Semantic Alerts

| Token | Color |
|---|---|
| `success` | green |
| `warning` | yellow |
| `error` | red |
| `info` | blue |

---

## Typography

All sizes are in pixels. Use `text_size(px(N))`.

| Scale | px | Use |
|---|---|---|
| `xs` | 10 | Badges, counts, column headers, timestamps |
| `sm` | 12 | List items, captions, input labels, tab labels |
| `base` | 13 | Body text, dropdowns, modal text |
| `md` | 14 | — (reserved, rarely needed) |
| `lg` | 15 | — (reserved) |
| `xl` | 16 | — (reserved) |

**Weights:**

| Weight | Use |
|---|---|
| `FontWeight::MEDIUM` | Standard labels, nav items |
| `FontWeight::SEMIBOLD` | Section titles, tab active state, input labels |
| `FontWeight::BOLD` | Method labels in chips |

Font family: Ubuntu Mono (monospace). All UI text uses this font.

---

## Spacing

8-point grid. Use `px(N)` matching one of these steps.

| Token | px | Use |
|---|---|---|
| `xs` | 4 | Icon gaps, tight inline spacing |
| `sm` | 8 | Standard inline gap, small padding |
| `md` | 12 | Panel padding, row `px` |
| `base` | 16 | Standard content padding |
| `lg` | 24 | Section gap |
| `xl` | 32 | Large section gap |
| `xxl` | 48 | — |

Common padding patterns:
- Row horizontal padding: `px(px(12.0))` (use `md`)
- Tab bar horizontal padding: `px(px(16.0))` (use `base`)
- Panel content padding: `px(px(12.0))` (use `md`)
- Inline icon-to-text gap: `gap(px(6.0))` or `gap(px(8.0))`

---

## Component Heights

All interactive components of the same tier share a height. **Never** mix tiers
within the same row.

| Constant | px | Use |
|---|---|---|
| `INPUT_XS` | 24 | Inline / micro inputs |
| `INPUT_SM` | 28 | Compact inputs, small icon buttons, action buttons |
| `INPUT_MD` | 32 | Standard inputs, medium buttons, KV rows, list rows |
| `PANEL_HEADER` | 32 | Section/collapsible headers |
| `TOOLBAR` | 40 | Toolbars, tab bars, top nav bars |
| `URL_BAR` | 64 | Primary URL bar |

Use `theme.sizes.*` (Pixels) or `theme::sizes::*` (f32) in code.

### Drag Handles

Resize dividers between sections:
- Height (horizontal) or width (vertical): `4px`
- Cursor: `cursor_row_resize` / `cursor_col_resize`
- Default: transparent (no visual)
- Hover: `accent.opacity(0.25)` background

---

## Border Radius

| Token | px | Use |
|---|---|---|
| `radius.sm` | 4 | Subtle rounding — badges, small chips |
| `radius.md` | 6 | Standard — inputs, buttons, cards |
| `radius.lg` | 8 | Pronounced — modals, dropdowns |
| `radius.xl` | 12 | Large — (reserved) |

> Note: GPUI has limited rounded-corner support in current use.
> Most components use sharp corners (`border_1()` only) for IDE aesthetic.

---

## Icons

Source: Lucide icon set (via `gpui-component-assets`). Use `icon(PATH, SIZE, COLOR)`.

| Constant | px | Use |
|---|---|---|
| `ICON_SM` | 11 | Inline chevrons, checkbox checks, tight rows |
| `ICON_MD` | 13 | Standard list icons, button icons |
| `ICON_LG` | 15 | Modal icons, prominent actions |

Icon color rules:
- Default state: `text_muted`
- Hover: `text_primary`
- Active / selected: `accent`
- Destructive hover: `status_client_error`

---

## Opacity System

| Level | Value | Use |
|---|---|---|
| disabled | 0.40 | Disabled element fill |
| muted | 0.60 | Muted/secondary text on colored bg |
| hover | 0.08 | Hover overlay on neutral surfaces |
| pressed | 0.12 | Active/pressed state, badge backgrounds |
| selected | 0.20 | Selected row background |

---

## Interaction States

### Hover

Most interactive elements use background-only hover:
```rust
.hover(|s| s.bg(theme.colors.bg_tertiary))
```

For colored (method/accent) elements:
```rust
.hover(|s| s.bg(method_color.opacity(0.15)))
```

For icon buttons that need text brightening:
```rust
.hover(|s| s.bg(theme.colors.bg_tertiary).text_color(theme.colors.text_primary))
```

Destructive hover (e.g., remove buttons):
```rust
.hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1))
            .text_color(theme.colors.status_client_error))
```

### Active / Selected

Selected list row or active tab:
- Background: `accent.opacity(0.15)`
- Text: `accent`
- Left border or bottom border: `accent` (full opacity, 2px)

### Focus

Text inputs and editable fields when focused:
- Border color: `border_focused` (= `accent`)
- No shadow (native GPUI doesn't support box-shadow)

### Disabled

- Text: `text_muted`
- Background: `bg_tertiary` or transparent
- Opacity: 0.4 on the whole element, or use `text_muted` + `border` color

---

## Components

### Badge / Pill

Small informational label. Never interactive.

```rust
div()
    .px(px(6.0)).py(px(2.0))
    .bg(color.opacity(0.12))
    .border_1().border_color(color.opacity(0.35))
    .text_size(px(10.0))
    .font_weight(FontWeight::MEDIUM)
    .text_color(color)
    .child(text)
```

`color` is semantic: `accent`, `method_*`, `status_*`, etc.

### Method Chip

Used in history rows and request save lists:

```rust
div()
    .min_w(px(44.0)).h(px(18.0))
    .flex().items_center().justify_center()
    .bg(method_color.opacity(0.12))
    .child(
        div()
            .text_size(px(10.0))
            .font_weight(FontWeight::BOLD)
            .text_color(method_color)
            .child(method_str)
    )
```

### Checkbox (KV Enable Toggle)

```rust
div()
    .size(px(16.0)).border_1()
    .when(enabled,  |s| s.bg(accent).border_color(accent))
    .when(!enabled, |s| s.border_color(border)
                         .hover(|s| s.border_color(text_muted)))
    .flex().items_center().justify_center()
    .when(enabled, |s| s.child(icon(ICON_CHECK, ICON_SM, bg_primary)))
```

### Remove Button (KV Row)

```rust
div()
    .size(px(28.0))
    .flex().items_center().justify_center()
    .when(can_remove, |el|
        el.cursor_pointer()
          .hover(|s| s.bg(status_client_error.opacity(0.1))
                      .text_color(status_client_error))
    )
    .child(icon(ICON_CLOSE, ICON_SM, text_muted))
```

### Add Row Button

Full-width dashed-look button at the bottom of KV tables:

```rust
div()
    .w_full().py(px(8.0))
    .border_1().border_color(border.opacity(0.5))
    .flex().items_center().justify_center()
    .cursor_pointer()
    .text_size(px(12.0)).text_color(text_muted)
    .hover(|s| s.bg(bg_tertiary).border_color(border).text_color(text_secondary))
    .child(label)
```

### Section Header (Collapsible)

Used for Collections, History, Script sections. Height = `PANEL_HEADER` (32px).

```rust
div()
    .id("section-header")
    .h(px(32.0)).w_full()
    .flex().items_center().justify_between()
    .px(px(12.0))
    .cursor_pointer()
    .hover(|s| s.bg(bg_tertiary))
    .on_click(...)
    .child(
        div().flex().items_center().gap(px(6.0))
            .child(if expanded { icon(CHEVRON_DOWN, ICON_SM, text_muted) }
                   else        { icon(CHEVRON_RIGHT, ICON_SM, text_muted) })
            .child(icon(section_icon, ICON_MD, text_muted))
            .child(div().text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(text_secondary)
                        .child(title))
    )
    // optional: badge on the right
```

### Toolbar / Tab Bar

Height = `TOOLBAR` (40px). `border_b_1()` with `border`.

Active tab:
- `font_weight(FontWeight::SEMIBOLD)`, `text_color(text_primary)`
- Bottom border 2px `accent`
- Optional: `bg(accent.opacity(0.08))`

Inactive tab:
- `font_weight(FontWeight::MEDIUM)`, `text_color(text_secondary)`
- Hover: `text_color(text_primary)`, `bg(bg_tertiary)`

### Content Toolbar (sub-tab action bars)

Rows that sit above content (e.g. the response body row with format badge and Copy
button). Height is implicit — whatever the tallest child needs. Always push action
buttons to the **right** using a `flex_1` spacer, **not** `justify_between`.

```rust
div()
    .w_full().flex().items_center()
    // Left: info content (badges, counts, labels)
    .child(div().flex().items_center().gap(px(8.0))
        .child(badge)
        .child(caption_text)
    )
    // flex_1 spacer — pushes everything after it to the right
    .child(div().flex_1())
    // Right: action button(s)
    .child(copy_button)
```

**Never use `justify_between`** for this pattern — GPUI's flex layout does not
reliably apply it when the container is inside an `overflow_scroll` region.

### Input Field

```rust
div()
    .h(px(32.0))             // INPUT_MD
    .flex().items_center()
    .px(px(10.0))
    .bg(bg_elevated)
    .border_1().border_color(if focused { border_focused } else { border })
    .hover(|s| s.border_color(text_muted))
    // text content inside
```

### Modal

Full-window overlay: `bg(overlay)` backdrop (`rgba(0,0,0,0.6)`).
Card: `bg(bg_elevated)`, `border_1()`, `border(border)`, padding `p(px(20.0))`, gap `gap(px(14.0))`.

Level icon (15px) + semibold title (13px) in a row.
Message body: 11px, `text_secondary`.
Buttons: right-aligned row, standard INPUT_SM height.

Primary action button: `bg(accent)`, `text_color(bg_primary)`, hover `bg(accent_hover)`.
Cancel button: `bg(bg_tertiary)`, `text_color(text_secondary)`, hover `bg(bg_elevated)`.

---

## Layout

### Main Window

```
┌──────────────┬─────────────────────────────┬───────────────┐
│              │  Tab bar (40px)             │               │
│  Explorer    │  URL bar (64px)             │  Response /   │
│  sidebar     │  Tab content (flex_1)       │  Codegen      │
│  (250px def) ├─────────────────────────────│  panel        │
│              │  Response panel             │  (400px def)  │
│              │  (320px default height)     │               │
└──────────────┴─────────────────────────────┴───────────────┘
Status bar (24px)
```

All split positions persist to `~/.config/protide/prefs.json`.

### Explorer Sidebar (left)

```
Header (40px) — workspace name + new-request button
─────────────────────────────────────────────────
Collections (collections_h, default 220px, resizable)
drag handle (4px)
History (flex_1, scrollable)
drag handle (4px, visible when env editor open)
Environment (auto / env_h when editor open)
```

### Request Scripts Tab

```
Pre-request header (36px, collapsible)
  Pre-request editor (script_pre_h, default 160px, resizable)
  drag handle (4px)
Post-response header (36px, collapsible)
  Post-response editor (script_post_h, default 160px, resizable)
  drag handle (4px)
Tests header (36px, collapsible)
  Tests editor (flex_1)
```

### Persistence Keys (`prefs.json`)

| Key | Default | What |
|---|---|---|
| `main.sidebar_width` | 250 | Explorer panel width |
| `main.request_height` | 320 | Request/response split |
| `main.mock_server_width` | 320 | Mock server panel width |
| `main.codegen_panel_width` | 400 | Code generation panel width |
| `explorer.collections_h` | 220 | Collections section height |
| `explorer.env_h` | 200 | Env editor height |
| `request.script_pre_h` | 160 | Pre-request editor height |
| `request.script_post_h` | 160 | Post-response editor height |

---

## Do / Don't

| Do | Don't |
|---|---|
| `theme.colors.accent` | Hard-code `rgb(0x4ade80)` |
| `theme.method_color("GET")` | Match on method string yourself |
| `theme.status_color(200)` | Match on status range yourself |
| Use `sizes::PANEL_HEADER` (32px) for section headers | Use arbitrary heights like `h(px(30.0))` |
| Use `ICON_SM/MD/LG` constants | Use arbitrary icon sizes like `size(px(12.0))` |
| `accent.opacity(0.12)` for badge bg | Full accent background on badges |
| `text_muted` for placeholder text | `text_secondary` or lower |
| `border_1()` + `border_color(border)` for default input border | Custom border widths |
| `div().flex_1()` spacer to right-align toolbar actions | `justify_between` (unreliable inside overflow_scroll) |
