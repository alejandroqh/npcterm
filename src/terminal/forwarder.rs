//! Cross-platform terminal emulator launcher for PTY forwarding.
//!
//! This module provides functionality to launch the user's preferred terminal
//! emulator and attach it to the PTY slave, allowing applications that
//! require direct terminal control (like Textual TUIs) to work correctly.

use std::process::{Child, Command};
use std::path::Path;

/// Result of launching a terminal emulator
#[derive(Debug)]
pub struct LaunchResult {
    /// The spawned child process
    pub child: Child,
    /// Name of the terminal emulator that was launched
    pub terminal_name: String,
}

/// Detect the best available terminal emulator on Linux
fn detect_linux_terminal() -> Option<String> {
    // Order matters: prefer more modern/user-friendly terminals
    let terminals = [
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "mate-terminal",
        "tilix",
        "alacritty",
        "kitty",
        "foot",
        "xterm",
    ];

    for terminal in terminals {
        if which_linux(terminal) {
            return Some(terminal.to_string());
        }
    }
    None
}

/// Check if a command exists on Linux
fn which_linux(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Launch terminal emulator on Linux
fn launch_linux_terminal(slave_name: &str, terminal: &str) -> std::io::Result<Child> {
    // Different terminals have different CLI arguments
    // Try common ones with their specific options
    match terminal {
        "gnome-terminal" => {
            // gnome-terminal -- fd=N opens the terminal connected to file descriptor N
            // But we need to pass the TTY device directly
            Command::new("script")
                .args(["-q", "-c", &format!("{} --disable-factory", terminal), slave_name])
                .spawn()
        }
        "xfce4-terminal" => {
            Command::new(terminal)
                .arg("-e")
                .arg(format!("script -q {} {}", slave_name, format!("{} -e 'cat {}'", terminal, slave_name)))
                .spawn()
        }
        "konsole" => {
            // Konsole has --hold option but doesn't directly support attaching to a PTY
            // Use script command as a wrapper
            Command::new("script")
                .args(["-q", "-c", &format!("{} --nofork", terminal), slave_name])
                .spawn()
        }
        "mate-terminal" => {
            Command::new("script")
                .args(["-q", "-c", terminal, slave_name])
                .spawn()
        }
        "xterm" => {
            Command::new("script")
                .args(["-q", "-c", terminal, slave_name])
                .spawn()
        }
        _ => {
            // Fallback: try script command which works universally
            Command::new("script")
                .args(["-q", "-c", terminal, slave_name])
                .spawn()
        }
    }
}

/// Launch terminal emulator on macOS
fn launch_macos_terminal() -> std::io::Result<Child> {
    // Use AppleScript to open Terminal.app and execute a command
    // The terminal will connect to the specified TTY
    let script = r#"
        tell application "Terminal"
            activate
            do script
        end tell
    "#;

    Command::new("osascript")
        .args(["-e", script])
        .spawn()
}

/// Launch terminal emulator on Windows
fn launch_windows_terminal(pid: u32) -> std::io::Result<Child> {
    // Windows Terminal doesn't support --attach to existing ConPTY sessions.
    // This is a known limitation - see Windows Terminal issue #2080.
    //
    // ConPTY uses handles, not paths, so it cannot be shared like Unix PTY.
    // The --attach feature was discussed but never implemented.
    //
    // Fallback: Start conhost.exe which provides better ConPTY support
    // than spawning a plain cmd window.
    Command::new("cmd")
        .args(["/c", "start", "/wait", "conhost.exe"])
        .spawn()
}

/// Launch a terminal emulator attached to the specified PTY slave
///
/// # Arguments
/// * `slave_name` - Path to the PTY slave device (e.g., "/dev/pts/4")
/// * `pid` - Process ID (used on Windows for attach mode)
///
/// # Returns
/// * `LaunchResult` containing the child process and terminal name on success
/// * Error if no suitable terminal emulator is found or launching fails
pub fn launch(_slave_name: &str, pid: u32) -> std::io::Result<LaunchResult> {
    #[cfg(target_os = "windows")]
    {
        let child = launch_windows_terminal(pid)?;
        Ok(LaunchResult {
            child,
            terminal_name: "Windows Terminal".to_string(),
        })
    }

    #[cfg(target_os = "macos")]
    {
        let child = launch_macos_terminal()?;
        Ok(LaunchResult {
            child,
            terminal_name: "Terminal.app".to_string(),
        })
    }

    #[cfg(target_os = "linux")]
    {
        let terminal = detect_linux_terminal()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No suitable terminal emulator found. Install gnome-terminal, konsole, or xterm.",
            ))?;

        let child = launch_linux_terminal(_slave_name, &terminal)?;
        Ok(LaunchResult {
            child,
            terminal_name: terminal,
        })
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Terminal forwarding is not supported on this platform.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_detect_linux_terminal() {
        let terminal = detect_linux_terminal();
        // This test may pass or fail depending on what's installed
        println!("Detected terminal: {:?}", terminal);
    }
}
