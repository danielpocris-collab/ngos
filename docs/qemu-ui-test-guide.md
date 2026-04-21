# NGOS UI - QEMU Test Guide

## 🚀 Cum să rulezi

### Varianta 1: PowerShell Script (Recomandat)

```powershell
cd C:\Users\pocri\OneDrive\Desktop\experiment\tooling\x86_64
.\run-qemu-ui-test.ps1
```

### Varianta 2: Manual

```powershell
# 1. Build
cargo build -p ngos-boot-x86_64

# 2. Rulează QEMU
& "C:\Program Files\qemu\qemu-system-x86_64.exe" `
  -machine pc `
  -m 512M `
  -cpu qemu64 `
  -drive if=pflash,format=raw,readonly=on,file=target\qemu\edk2-x86_64-code.fd `
  -drive if=pflash,format=raw,file=target\qemu\edk2-x86_64-vars-ui.fd `
  -drive if=none,id=esp,format=raw,file=target\qemu\limine-uefi-ui.img `
  -device virtio-blk-pci,drive=esp,bootindex=1 `
  -display sdl `
  -no-reboot
```

---

## 👁️ Ce Vei Vedea

### Timeline Boot Sequence:

```
┌─────────────────────────────────────────┐
│  T+0.0s: Ecran negru                    │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  T+0.5s: Apare logo NGOS                │
│                                         │
│         ┌──────────────┐                │
│         │      N       │                │
│         │   (gradient) │                │
│         └──────────────┘                │
│                                         │
│            NGOS                         │
│       NEXT GEN OS                       │
│                                         │
│      ━━━━━━━━━━━━━━━━    0%            │
│                                         │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  T+1.0s: Apare primul mesaj             │
│                                         │
│         ┌──────────────┐                │
│         │      N       │                │
│         └──────────────┘                │
│            NGOS                         │
│      ━━━━━━━━━━━━━━━━    25%           │
│                                         │
│  ✓ Loading kernel core...               │
│                                         │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  T+1.5s: Al doilea mesaj                │
│                                         │
│         ┌──────────────┐                │
│         │      N       │  (pulse)       │
│         └──────────────┘                │
│            NGOS                         │
│      ━━━━━━━━━━━━━━━━━━━━  50%         │
│                                         │
│  ✓ Loading kernel core...               │
│  ✓ Initializing graphics subsystem...   │
│                                         │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  T+2.0s: Al treilea mesaj               │
│                                         │
│         ┌──────────────┐                │
│         │      N       │  (pulse)       │
│         └──────────────┘                │
│            NGOS                         │
│      ━━━━━━━━━━━━━━━━━━━━━━━━━━  75%   │
│                                         │
│  ✓ Loading kernel core...               │
│  ✓ Initializing graphics subsystem...   │
│  ✓ Mounting file system...              │
│                                         │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  T+2.5s: Al patrulea mesaj              │
│                                         │
│         ┌──────────────┐                │
│         │      N       │  (pulse)       │
│         └──────────────┘                │
│            NGOS                         │
│      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 100%│
│                                         │
│  ✓ Loading kernel core...               │
│  ✓ Initializing graphics subsystem...   │
│  ✓ Mounting file system...              │
│  ✓ Starting user interface...           │
│                                         │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  T+3.0s: Al cincilea mesaj              │
│                                         │
│         ┌──────────────┐                │
│         │      N       │  (pulse)       │
│         └──────────────┘                │
│            NGOS                         │
│      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 100%│
│                                         │
│  ✓ Loading kernel core...               │
│  ✓ Initializing graphics subsystem...   │
│  ✓ Mounting file system...              │
│  ✓ Starting user interface...           │
│  ✓ Welcome to NGOS v0.1.0               │
│                                         │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  T+3.5s: Boot screen dispare (fade)     │
│                                         │
│  Apare desktop-ul NGOS:                 │
│                                         │
│  ┌──────────────────────────────────┐   │
│  │  🏠 NGOS  [📁][🌐][🧮][💻][🎵]   │   │
│  │                                   │   │
│  │     [Desktop gol - ready]         │   │
│  │                                   │   │
│  └──────────────────────────────────┘   │
│  ┌──────────────────────────────────┐   │
│  │ 🏠 [📁] [🌐] [🧮] [💻] [🎵]  🌐🔊🔋 │   │
│  └──────────────────────────────────┘   │
│                                         │
└─────────────────────────────────────────┘
```

---

## 🎨 Culori și Efecte

### Logo NGOS:
- **Gradient**: Cyan (#00d4ff) → Purple (#7b2cbf)
- **Mărime**: 150x150px
- **Glow**: Cyan shadow 60px
- **Pulse**: Se mărește și se micșorează (1.0x → 1.05x)

### Progress Bar:
- **Lungime**: 400px
- **Înălțime**: 6px
- **Culoare**: Gradient cyan→purple
- **Glow**: Cyan shadow 20px

### Background:
- **Gradient**: Dark navy (#0a0a0f) → Medium (#1a1a2e) → Dark teal (#16213e)

### Text:
- **NGOS**: Gradient cyan→purple, 32px, bold
- **NEXT GEN OS**: Gray (#94a3b8), 14px
- **Messages**: Gray (#94a3b8), 12px, monospace

---

## ⌨️ Controale în QEMU

| Acțiune | Combinație |
|---------|------------|
| Închide QEMU | Ctrl+A, apoi X |
| Fullscreen | Ctrl+Alt+F |
| Capture mouse | Click în fereastră |
| Eliberează mouse | Ctrl+Alt |

---

## 🐛 Debug

### Dacă nu pornește:

1. **Verifică build-ul**:
   ```powershell
   cargo check -p ngos-boot-x86_64
   ```

2. **Verifică QEMU**:
   ```powershell
   & "C:\Program Files\qemu\qemu-system-x86_64.exe" --version
   ```

3. **Verifică firmware UEFI**:
   ```powershell
   Test-Path "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
   ```

### Serial Log:

După rulare, verifică:
```
target\qemu\serial-ui.log
```

Acolo vei vedea mesajele de boot text.

---

## 📊 Ce ar trebui să se întâmple:

1. ✅ QEMU pornește
2. ✅ Logo NGOS apare central
3. ✅ Progress bar se umple (0% → 100%)
4. ✅ 5 mesaje apar pe rând
5. ✅ Boot screen dispare după 3.5s
6. ✅ Desktop-ul apare cu taskbar

## ❌ Ce ar putea merge prost:

1. **Ecran negru** - Firmware UEFI lipsă
2. **Eroare la boot** - Kernel build fail
3. **Nu apare UI** - ngos-ui nu e integrat corect
4. **QEMU se închide** - Memory prea mic (minim 512MB)

---

## 🎯 Next Steps After Successful Boot:

1. Adaugă input handling (mouse/keyboard)
2. Implementează window dragging
3. Adaugă font rendering real
4. Creează demo apps (calculator, terminal)

**Good luck!** 🚀
