#Requires AutoHotkey v2.0
#SingleInstance Force

if A_Args.Length < 1 {
    MsgBox "Usage:`n  AutoHotkey64.exe command-runner.ahk ""command to run""", "command-runner", "Icon!"
    ExitApp 2
}

command := A_Args[1]
message := A_Args.Length >= 2 ? A_Args[2] : "command finished"

hwnd := WinExist("A")
if !hwnd {
    MsgBox "No active window found.", "command-runner", "Icon!"
    ExitApp 1
}

RunWait(A_ComSpec ' /c "' command '"')

if !WinExist("ahk_id " hwnd) {
    MsgBox "The original PowerShell window is no longer available.", "command-runner", "Icon!"
    ExitApp 1
}

WinActivate("ahk_id " hwnd)
WinWaitActive("ahk_id " hwnd, , 2)
A_Clipboard := message
Sleep 150
Send "^v{Enter}"
ExitApp
