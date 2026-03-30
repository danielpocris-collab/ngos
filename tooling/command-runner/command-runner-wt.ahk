#Requires AutoHotkey v2.0
#SingleInstance Force

if A_Args.Length < 1 {
    MsgBox "Usage:`n  AutoHotkey64.exe command-runner-wt.ahk ""command to run""", "command-runner-wt", "Icon!"
    ExitApp 2
}

command := A_Args[1]
message := A_Args.Length >= 2 ? A_Args[2] : "command finished"

hwnd := WinExist("A")
if !hwnd {
    MsgBox "No active window found.", "command-runner-wt", "Icon!"
    ExitApp 1
}

winClass := WinGetClass("ahk_id " hwnd)
if winClass != "CASCADIA_HOSTING_WINDOW_CLASS" && winClass != "PseudoConsoleWindow" {
    MsgBox "The active window is not a Windows Terminal / ConPTY-hosted console.`nClass: " winClass, "command-runner-wt", "Icon!"
    ExitApp 1
}

RunWait(A_ComSpec ' /c "' command '"')

if !WinExist("ahk_id " hwnd) {
    MsgBox "The original terminal window is no longer available.", "command-runner-wt", "Icon!"
    ExitApp 1
}

WinActivate("ahk_id " hwnd)
WinWaitActive("ahk_id " hwnd, , 2)
A_Clipboard := message
Sleep 200
Send "^+v"
Sleep 120
Send "{Enter}"
ExitApp
