# NGOS UI/UX Design System

## Overview

Acest document prezintă sistemul de design UI/UX pentru Next Gen OS, incluzând toate componentele grafice implementate.

---

## 🎨 Paleta de Culori

### Culori Primare

```
┌─────────────────────────────────────────────────────────────┐
│  Primary Dark        ██████ #0b111a  (Background principal) │
│  Primary Medium      ██████ #1a2333  (Top bar, panels)      │
│  Primary Light       ██████ #2a3448  (Focused elements)     │
│  Accent Blue         ██████ #4b92e8  (Active states)        │
│  Accent Green        ██████ #2fb08b  (Success/positive)     │
│  Accent Orange       ██████ #c87331  (Warning/warmth)       │
│  Error Red           ██████ #e84b4b  (Close/danger)         │
└─────────────────────────────────────────────────────────────┘
```

### Culori Semantice

| Utilizare | Culoare | Hex | RGBA |
|-----------|---------|-----|------|
| Background | Dark Navy | `#0b111a` | `rgba(11,17,26,255)` |
| Surface | Charcoal | `#1f2a3d` | `rgba(31,42,61,255)` |
| Border | Slate | `#2a3448` | `rgba(42,52,72,255)` |
| Focus | Ocean Blue | `#4b92e8` | `rgba(75,146,232,255)` |
| Success | Emerald | `#2fb08b` | `rgba(47,176,139,255)` |
| Warning | Amber | `#c87331` | `rgba(200,115,49,255)` |
| Error | Crimson | `#e84b4b` | `rgba(232,75,75,255)` |

### Culori Glassmorphism

```
Translucency levels:
  - Light:  rgba(246,251,255, 0x10) = 16/255  (7% opacity)
  - Medium: rgba(246,251,255, 0x14) = 20/255  (8% opacity)
  - Strong: rgba(246,251,255, 0x12) = 18/255  (7% opacity)
```

---

## 🖼️ Boot Desktop Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│  TOP BAR (60px height) - #1a2333                                     │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │ 🍎 NGOS  │  File  Edit  View  Go  Help  │          🕐 12:30  │  │
│  └────────────────────────────────────────────────────────────────┘  │
├──────────┬───────────────────────────────────┬───────────────────────┤
│          │                                   │                       │
│ SIDEBAR  │         MAIN WINDOW               │     INSPECTOR         │
│ (240px)  │         (Canvas)                  │     (240px)           │
│          │                                   │                       │
│ #131b28  │  ┌─────────────────────────────┐  │  ┌─────────────────┐  │
│          │  │  Title Bar (blue accent)    │  │  │  Properties     │  │
│ 📁 Files │  ├─────────────────────────────┤  │  ├─────────────────┤  │
│ 🎨 Apps  │  │                             │  │  │  • Color        │  │
│ ⚙️  Settings│  │    [Blue Card]  [Green]    │  │  │  • Size         │  │
│ 🔍 Search│  │                             │  │  │  • Position     │  │
│          │  │    [Orange Card]            │  │  │  • Transform    │  │
│ ──────── │  │                             │  │  │                 │  │
│          │  │                             │  │  └─────────────────┘  │
│ Volumes: │  └─────────────────────────────┘  │                       │
│  📀 HDD  │                                   │  ┌─────────────────┐  │
│  💾 SSD  │  ┌─────────────────────────────┐  │  │  Layers         │  │
│  🌐 Net  │  │     BOTTOM WINDOW           │  │  ├─────────────────┤  │
│          │  │     (Terminal/Console)      │  │  │  • Layer 1      │  │
│          │  │  $ ngos-shell> _            │  │  │  • Layer 2      │  │
│          │  │                             │  │  │  • Layer 3      │  │
│          │  └─────────────────────────────┘  │  └─────────────────┘  │
│          │                                   │                       │
├──────────┴───────────────────────────────────┴───────────────────────┤
│  DOCK (78px height) - #161e2d                                        │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  🍎  📁  🌐  🎨  ⚙️  🗑️                                       │  │
│  └────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

### Dimensions (Responsive)

| Element | Formula | Min | Example (1920x1080) |
|---------|---------|-----|---------------------|
| Top Bar | `height / 13` | 60px | 83px |
| Dock | `height / 11` | 78px | 98px |
| Sidebar | `width / 5` | 240px | 384px |
| Inspector | `width / 4` | 240px | 480px |
| Margin | `width / 42` | 24px | 45px |
| Gap | `margin / 2` | 16px | 22px |

---

## 🪟 Window Chrome Design

```
┌─────────────────────────────────────────────────────────┐
│ ╔═══════════════════════════════════════════════════╗   │
│ ║ ▐█▌  Window Title                    ─  □  ✕     ║   │
│ ║ ──────────────────────────────────────────────────║   │
│ ║                                                   ║   │
│ ║                                                   ║   │
│ ║              Content Area                         ║   │
│ ║                                                   ║   │
│ ║                                                   ║   │
│ ╚═══════════════════════════════════════════════════╝   │
│   ═══════════════════════════════════════════════       │
│           Shadow (blur: 16px, alpha: 0x60)              │
└─────────────────────────────────────────────────────────┘
```

### Window States

```
NORMAL:                    FOCUSED:
┌─────────────┐            ┌─────────────┐
│ #1a2333     │            │ #2a3448     │
│ ────────    │            │ ████ #4b92e8│
│  Content    │            │  Content    │
└─────────────┘            └─────────────┘

MINIMIZED:                 MAXIMIZED:
┌─────────────┐            ┌───────────────────────────┐
│  ▔▔▔▔▔▔▔    │            │                           │
│  App Name   │            │  Full screen content      │
└─────────────┘            │                           │
                           └───────────────────────────┘
```

### Window Buttons

```
┌─────────────────────────────────────────┐
│  Title Bar                              │
│                              ┌─┐ ┌─┐ ┌─┐│
│                              │─│ │□│ │✕││
│                              └─┘ └─┘ └─┘│
│                               │   │   │ │
│                               │   │   └─► Close (#e84b4b)
│                               │   └─────► Maximize (#4be84b)
│                               └─────────► Minimize (#4b92e8)
└─────────────────────────────────────────┘
```

---

## ✨ Effects Pipeline

### 1. Drop Shadow

```
┌─────────────────┐
│                 │
│   ┌─────────┐   │     Shadow properties:
│   │ Element │   │       • offset-x: 0px
│   │         │   │       • offset-y: 4px
│   └─────────┘   │       • blur: 8-16px
│     ▓▓▓▓▓▓▓     │       • spread: 0-8px
│   ▓▓▓▓▓▓▓▓▓▓▓   │       • color: rgba(0,0,0,0.4-0.6)
└─────────────────┘
```

### 2. Glassmorphism (Backdrop Blur)

```
┌─────────────────────────────────┐
│  ████████████████████████████   │  Layer stack (top to bottom):
│  ██  Glass Panel (blurred) ██   │    1. GaussianBlur (radius: 10-20)
│  ██  rgba(246,251,255,0x10) ██   │    2. Backdrop (opacity: 0x8E)
│  ████████████████████████████   │    3. Tint rect (rgba + alpha)
│      ░░░░░░░░░░░░░░░░░░░░░      │    4. Content
│    Background (image/color)     │
└─────────────────────────────────┘
```

### 3. Gradient Fills

```
Horizontal:              Vertical:              Radial:
┌──────────────┐         ┌──────────────┐       ◉
│ ████ gradient │        │ ▓▓▓▓▓▓▓▓▓▓▓▓ │      ◉◉◉
│ → right      │        │ ▓▓▓ gradient ▓▓ │     ◉◉◉◉◉
│ #112233→#445566│       │ ▓▓▓▓▓▓▓▓▓▓▓▓ │      ◉◉◉
└──────────────┘         │ ↓ down       │       ◉
                         └──────────────┘
```

### 4. Temporal Animation (Pulse)

```
Alpha pulse over time (stride = 2):

Tick 0:   ████████  alpha = base (60)
Tick 1:   ██████████  alpha = base + 1 (61)
Tick 2:   ████████████  alpha = base + 2 (62)
...
Tick 255: ██████████████████████████████  alpha = peak (100)
Tick 256: ████████████████████████████  alpha = peak - 1 (99)
...
Tick 511: ████████  alpha = base (60)
          └─────────────────────────────►
                    Time (512 ticks cycle)
```

---

## 🖱️ Input Handling

### Mouse Cursor States

```
DEFAULT:              HOVER:               CLICK:
  │                    ╱│╲                  ╱│╲
  │                   ╱ │ ╲                ╱ │ ╲
 ─┴─                 ─┴─┴─                ─┴─┴─
                     (enlarged)           (pressed)
```

### Keyboard Modifiers

```
Modifier keys (bitmask):

  SHIFT   CONTROL   ALT   SUPER
    │        │       │      │
    ▼        ▼       ▼      ▼
  ┌────────────────────────┐
  │ 0  0  0  0  S  C  A  S │  ← 8-bit mask
  └────────────────────────┘
  
Example: Ctrl+Shift+A = 0b00000011 = 0x03
```

---

## 🎯 Widget Toolkit (Planned)

### Button States

```
NORMAL:                 HOVER:                  PRESSED:
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│   Button    │         │   Button    │         │   Button    │
└─────────────┘         └─────────────┘         └─────────────┘
  #4b92e8                 #5ba3f9                 #3a82d8
  (flat)                  (brighter)              (darker, inset)
```

### Text Input

```
┌─────────────────────────────────┐
│  Label                          │
│ ┌─────────────────────────────┐ │
│ │ Enter text...           │   │ │  ← Blinking cursor
│ └─────────────────────────────┘ │
│   #1f2a3d (background)          │
│   #4b92e8 (focus border)        │
└─────────────────────────────────┘
```

### Checkbox & Radio

```
CHECKBOX:               RADIO:
┌─────┐  Label          ◉  Label
│ ✓   │                 ●
└─────┘                 
 (checked)              (selected)

┌─────┐  Label          ○  Label
│     │                 ○
└─────┘                 
 (unchecked)            (unselected)
```

---

## 📐 Spacing System

```
Base unit: 8px

Spacing scale:
  xs:  4px   (0.5x)   ██
  sm:  8px   (1x)     ████
  md:  16px  (2x)     ████████
  lg:  24px  (3x)     ████████████
  xl:  32px  (4x)     ████████████████
  2xl: 48px  (6x)     ████████████████████████
```

### Layout Margins

```
┌────────────────────────────────────┐
│  xl (32px)                         │
│  ┌──────────────────────────────┐  │
│  │                              │  │
│  │  lg (24px)                   │  │
│  │  ┌────────────────────────┐  │  │
│  │  │                        │  │  │
│xl│  │md    Content       md  │  │xl
│  │  │                        │  │  │
│  │  └────────────────────────┘  │  │
│  │                              │  │
│  └──────────────────────────────┘  │
│  xl (32px)                         │
└────────────────────────────────────┘
```

---

## 🎭 Layer System (Z-Index)

```
Layer Stack (bottom to top):

  6  ┌─────────────────────────┐  ← Modal Dialogs
     │      MODAL              │
  5  ├─────────────────────────┤  ← Popovers/Tooltips
     │    POPUP                │
  4  ├─────────────────────────┤  ← Focused Window
     │  ┌─────────────────┐    │
  3  │  │   WINDOW (↑)    │    │  ← Regular Windows
     │  │                 │    │
  2  │  └─────────────────┘    │
     ├─────────────────────────┤  ← Dock/Taskbar
  1  │      DOCK               │
     ├─────────────────────────┤
  0  │      DESKTOP BG         │  ← Desktop Background
     └─────────────────────────┘
```

---

## 🖼️ FrameScript Examples

### Simple Rectangle

```framescript
surface=1280x720
frame=example-001
queue=graphics
present-mode=mailbox
completion=wait-present
clear=#0b111aff
rect=100,100,400,300,#4b92e8ff
```

### Window with Shadow

```framescript
surface=1280x720
frame=window-example
queue=graphics
present-mode=mailbox
completion=wait-present
clear=#0b111aff

# Window shadow
shadow-rect=104,104,400,300,16,#00000060

# Window background
rect=100,100,400,300,#1f2a3dff

# Title bar
rect=100,100,400,32,#2a3448ff

# Title bar accent
rect=100,100,6,32,#4b92e8ff

# Window content
rect=100,132,400,268,#1f2a3dff
```

### Gradient Background

```framescript
surface=1920x1080
frame=desktop-bg
queue=graphics
present-mode=mailbox
completion=wait-present

# Gradient background
gradient-rect=0,0,1920,1080,#0b111aff,#141d2fff,#101825ff,#060b12ff
```

---

## 📊 Component Inventory

| Component | Crate | Status | Tests |
|-----------|-------|--------|-------|
| Input Handling | `ngos-input-translate` | ✅ Complete | 18 |
| Window Manager | `ngos-window-manager` | ✅ Complete | 12 |
| Compositor | `ngos-compositor` | ✅ Complete | 32 |
| Effects Pipeline | `ngos-effects-pipeline` | ✅ Complete | 35 |
| Scene Graph | `ngos-scene-graph` | ✅ Complete | 45 |
| 3D Renderer | `ngos-render3d` | ✅ Complete | 119 |
| **Total** | | **✅ 6/6** | **261 teste** |

---

## 🎨 Design Tokens

```rust
// Colors
const COLOR_BACKGROUND: RgbaColor = RgbaColor { r: 0x0b, g: 0x11, b: 0x1a, a: 0xff };
const COLOR_SURFACE: RgbaColor    = RgbaColor { r: 0x1f, g: 0x2a, b: 0x3d, a: 0xff };
const COLOR_ACCENT: RgbaColor     = RgbaColor { r: 0x4b, g: 0x92, b: 0xe8, a: 0xff };

// Spacing
const SPACING_XS: u32 = 4;
const SPACING_SM: u32 = 8;
const SPACING_MD: u32 = 16;
const SPACING_LG: u32 = 24;
const SPACING_XL: u32 = 32;

// Typography (planned)
const FONT_SIZE_XS: u32 = 10;
const FONT_SIZE_SM: u32 = 12;
const FONT_SIZE_MD: u32 = 14;
const FONT_SIZE_LG: u32 = 18;
const FONT_SIZE_XL: u32 = 24;

// Border Radius
const RADIUS_SM: u32 = 4;
const RADIUS_MD: u32 = 8;
const RADIUS_LG: u32 = 16;
const RADIUS_XL: u32 = 24;

// Shadows
const SHADOW_COLOR: RgbaColor = RgbaColor { r: 0x00, g: 0x00, b: 0x00, a: 0x60 };
const SHADOW_BLUR_SM: u32 = 4;
const SHADOW_BLUR_MD: u32 = 8;
const SHADOW_BLUR_LG: u32 = 16;
```

---

## 📝 Notes

- Toate culorile sunt definite în spațiul sRGB
- Opacity values: 0x00 (0%) - 0xFF (100%)
- FrameScript este limbajul de renderizare proprietar
- Compoziția este realizată prin `ngos-compositor`
- Efectele sunt compuse prin `ngos-effects-pipeline`
- Input-ul este gestionat de `ngos-input-translate`
- Ferestrele sunt administrate de `ngos-window-manager`
