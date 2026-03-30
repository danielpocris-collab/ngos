use std::env;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let mut paste_after = None;

    while let Some(flag) = args.first() {
        if flag == "--paste-after" {
            args.remove(0);
            let Some(message) = args.first().cloned() else {
                eprintln!("error: missing message after `--paste-after`");
                return ExitCode::from(2);
            };
            paste_after = Some(message);
            args.remove(0);
            continue;
        }
        break;
    }

    if args.is_empty() {
        eprintln!(
            "usage: ngos-command-runner [--paste-after <message>] -- <command> [args...]\nexample: ngos-command-runner --paste-after \"next task\" -- cargo test -p ngos-kernel-core"
        );
        return ExitCode::from(2);
    }

    if args.first().is_some_and(|arg| arg == "--") {
        args.remove(0);
    }

    if args.is_empty() {
        eprintln!("error: missing command after `--`");
        return ExitCode::from(2);
    }

    let program = args.remove(0);
    let status = match Command::new(&program).args(&args).status() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("failed to start `{program}`: {error}");
            return ExitCode::from(1);
        }
    };

    println!();
    println!("execution finished");

    if let Some(message) = paste_after.as_deref()
        && let Err(error) = paste_into_active_cli(message)
    {
        eprintln!("failed to paste follow-up text into active CLI: {error}");
    }

    if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn paste_into_active_cli(message: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        paste_into_active_terminal(message)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = message;
        Err(String::from(
            "`--paste-after` is currently implemented only on Windows",
        ))
    }
}

#[cfg(target_os = "windows")]
fn paste_into_active_terminal(message: &str) -> Result<(), String> {
    let escaped = message.replace('\'', "''");
    let script = format!(
        "$sig='[System.Runtime.InteropServices.DllImport(\"kernel32.dll\")] public static extern System.IntPtr GetConsoleWindow(); [System.Runtime.InteropServices.DllImport(\"user32.dll\")] public static extern bool SetForegroundWindow(System.IntPtr hWnd);'; Add-Type -Namespace Win32 -Name Native -MemberDefinition $sig; $wshell = New-Object -ComObject WScript.Shell; Set-Clipboard -Value '{escaped}'; $hwnd=[Win32.Native]::GetConsoleWindow(); if ($hwnd -eq [System.IntPtr]::Zero) {{ throw 'GetConsoleWindow failed' }}; [void][Win32.Native]::SetForegroundWindow($hwnd); Start-Sleep -Milliseconds 150; $wshell.SendKeys('^v~')"
    );
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|error| error.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("powershell exited with status {status}"))
            }
        })
}
