# UI Improvements - Minimal Aesthetic

## Overview
Comprehensive list of UI improvements needed to achieve a clean, minimal aesthetic throughout the application.

---

## Response Panel Issues

### 1. Response Header Height & Styling
**File:** `crates/api-dash/src/ui/panels/response.rs:274`
- **Current:** Height 44px, uses bg_secondary, has "↓" icon
- **Target:** Height 40px, use bg_primary, simplify styling
- **Priority:** 5

### 2. Response Tabs with Pipe Separators
**File:** `crates/api-dash/src/ui/panels/response.rs:416-519`
- **Current:** Rounded-top tabs with backgrounds, icons ("{ }", "≡", "🍪")
- **Target:** Simple text with pipe separators like request tabs
- **Priority:** 1 (Biggest visual impact)

### 3. Status Info Badges
**File:** `crates/api-dash/src/ui/panels/response.rs:308-414`
- **Current:** Heavy decoration with icons, rounded backgrounds, borders
- **Target:** Simpler status display without excessive decoration
- **Priority:** 11

---

## Request Panel Issues

### 4. Auth Type Selector Pills
**File:** `crates/api-dash/src/ui/panels/request/render.rs:1617-1657`
- **Current:** Pill-style buttons with bg_tertiary, emoji icons (🎫👤🔑), rounded(8px)
- **Target:** Simple text buttons or minimal dropdown
- **Priority:** 2

### 5. Body Type Selector Pills
**File:** `crates/api-dash/src/ui/panels/request/render.rs:1027-1073`
- **Current:** Pill-style with bg_tertiary, p(3px), rounded(6px)
- **Target:** Match auth selector simplification
- **Priority:** 2

### 6. Table Headers - Font Weight
**File:** `crates/api-dash/src/ui/panels/request/render.rs:620-669, 1088-1124`
- **Current:** SEMIBOLD weight, uppercase "KEY"/"VALUE"/"TYPE"
- **Target:** Normal weight, more subtle styling
- **Priority:** 4

### 7. Padding Consistency
**File:** Multiple locations in `render.rs`
- **Current:** Tabs use p(20px), URL bar uses px(16px)
- **Target:** All use 16px consistently
- **Locations:**
  - Line 613: Params tab `.p(px(20.0))`
  - Line 849: Headers tab `.p(px(20.0))`
  - Line 1008: Body tab `.p(px(20.0))`
  - Auth tab, Scripts tab, etc.
- **Priority:** 3

### 8. Add Buttons Border
**File:** `crates/api-dash/src/ui/panels/request/render.rs:770-795` and similar
- **Current:** Has borders, rounded(6px), padding
- **Target:** Borderless, simpler text buttons
- **Priority:** 8

### 9. Method Dropdown Overlay
**File:** `crates/api-dash/src/ui/panels/request/render.rs` (need to locate)
- **Current:** Likely has shadows and heavy styling
- **Target:** Match simplified mode dropdown
- **Priority:** 7

### 10. Mode Dropdown Overlay Shadow
**File:** `crates/api-dash/src/ui/panels/request/render.rs:377-437`
- **Current:** Uses shadow_lg(), selection dots
- **Target:** Remove shadow, simplify selection indicators
- **Priority:** 7

### 11. Checkboxes Styling
**File:** `crates/api-dash/src/ui/panels/request/render.rs:691-719, 1147-1179`
- **Current:** 16px size, colored background when checked, checkmark
- **Target:** More subtle styling
- **Priority:** 9

### 12. Border Radius Consistency
**File:** Multiple locations throughout render.rs
- **Current:** Mix of 8px, 6px, 4px
- **Target:** Standardize to 4px or max 6px
- **Priority:** 6

### 13. Remove Button Hover States
**File:** `crates/api-dash/src/ui/panels/request/render.rs:747-765`
- **Current:** Hover shows red background with status_client_error
- **Target:** More subtle hover, less prominent
- **Priority:** 10

### 14. Count Badges Decoration
**File:** `crates/api-dash/src/ui/panels/request/render.rs:656-665, 1076-1084`
- **Current:** px(6), py(2), rounded(8), colored backgrounds
- **Target:** Simpler text display
- **Priority:** 11

---

## Priority Order

1. **Response tabs** → pipe separators (biggest visual impact)
2. **Auth/Body type selectors** → remove pills and icons
3. **Padding consistency** → all 20px → 16px
4. **Table headers** → remove semibold weight
5. **Response header height** → 44px → 40px
6. **Border radius** → standardize to 4-6px
7. **Method/Mode dropdown overlays** → remove shadows
8. **Add buttons** → remove borders
9. **Checkboxes** → simplify styling
10. **Remove button hovers** → less prominent
11. **Count badges** → simplify
12. **Loading/error states** → reduce decoration

---

## Implementation Notes

- Follow the minimal aesthetic from project plan
- Reduce visual noise, focus on content
- Consistent spacing (16px padding)
- Consistent border radius (4-6px max)
- Remove unnecessary icons and decorations
- Simpler color usage
- Less prominent interactive states
