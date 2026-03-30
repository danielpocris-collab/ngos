# Command Runner

`command-runner.ahk` este varianta recomandata pentru PowerShell pe Windows.

Folosire:

```powershell
AutoHotkey64.exe .\tooling\command-runner\command-runner.ahk "cargo test -p ngos-kernel-core"
```

Cu mesaj custom:

```powershell
AutoHotkey64.exe .\tooling\command-runner\command-runner.ahk "cargo test -p ngos-kernel-core" "next task"
```

Ce face:

- retine fereastra PowerShell activa
- ruleaza comanda data
- readuce aceeasi fereastra in foreground
- pune mesajul in clipboard
- trimite `Ctrl+V` si `Enter`

Cerinta:

- AutoHotkey v2 instalat pe Windows
