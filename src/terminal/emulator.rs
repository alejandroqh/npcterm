use super::ansi_handler::AnsiHandler;
use super::grid::TerminalGrid;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{BufWriter, Read, Write};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;
use std::time::Instant;
use vte::Parser;

#[cfg(not(target_os = "linux"))]
use std::process::Command;

/// Allowed terminal sizes
const ALLOWED_SIZES: [(usize, usize); 4] = [(80, 24), (120, 40), (160, 40), (200, 50)];
const DEFAULT_MAX_SCROLLBACK: usize = 10_000;

/// Check if a shell can be found
fn shell_exists(name: &str) -> bool {
    let path = std::path::Path::new(name);
    if path.exists() {
        return true;
    }
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if dir.join(name).exists() {
                return true;
            }
        }
    }
    false
}

/// Terminal emulator: PTY spawning, I/O, process state
pub struct TerminalEmulator {
    /// Terminal grid (screen buffer) — owned directly, no Arc/Mutex needed
    pub grid: TerminalGrid,
    /// VTE parser
    parser: Parser,
    /// PTY master (for reading/writing)
    #[allow(dead_code)]
    pty_master: Box<dyn MasterPty + Send>,
    /// PTY writer (buffered)
    writer: BufWriter<Box<dyn Write + Send>>,
    /// Child process handle
    child: Box<dyn Child + Send>,
    /// Channel to receive data from PTY reader thread
    rx: Receiver<Vec<u8>>,
    /// Last time output was received from PTY
    pub last_output_time: Option<Instant>,
    /// Cached exit status
    pub exit_code: Option<i32>,
}

impl TerminalEmulator {
    /// Create a new terminal emulator
    ///
    /// # Arguments
    /// * `cols` - Number of columns (must be 80 or 120)
    /// * `rows` - Number of rows (must be 24 or 40)
    /// * `shell` - Optional shell path. None = system default
    pub fn new(cols: usize, rows: usize, shell: Option<&str>) -> std::io::Result<Self> {
        // Validate size
        if !ALLOWED_SIZES.contains(&(cols, rows)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid terminal size {}x{}. Allowed: 80x24, 120x40, 160x40, 200x50",
                    cols, rows
                ),
            ));
        }

        let pty_system = native_pty_system();

        let pty_pair = pty_system
            .openpty(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(std::io::Error::other)?;

        let mut cmd = if let Some(shell_path) = shell {
            if shell_exists(shell_path) {
                CommandBuilder::new(shell_path)
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Shell '{}' not found", shell_path),
                ));
            }
        } else {
            CommandBuilder::new_default_prog()
        };

        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("PROMPT_EOL_MARK", "");
        cmd.env("PROMPT_SP", "");

        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(std::io::Error::other)?;

        let pty_master = pty_pair.master;

        let mut reader = pty_master
            .try_clone_reader()
            .map_err(std::io::Error::other)?;

        let writer = BufWriter::new(pty_master.take_writer().map_err(std::io::Error::other)?);

        let (tx, rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = sync_channel(64);

        thread::spawn(move || {
            let mut buffer = vec![0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let grid = TerminalGrid::new(cols, rows, DEFAULT_MAX_SCROLLBACK);
        let parser = Parser::new();

        Ok(Self {
            grid,
            parser,
            pty_master,
            writer,
            child,
            rx,
            last_output_time: None,
            exit_code: None,
        })
    }

    /// Read output from PTY and process through ANSI parser.
    /// Returns true if child is still running.
    pub fn process_output(&mut self) -> bool {
        let mut chunks = Vec::new();
        let mut running = true;

        loop {
            match self.rx.try_recv() {
                Ok(data) => chunks.push(data),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    running = false;
                    break;
                }
            }
        }

        if !chunks.is_empty() {
            self.last_output_time = Some(Instant::now());
            let mut handler = AnsiHandler::new(&mut self.grid);
            for data in chunks {
                self.parser.advance(&mut handler, &data);
            }
        }

        // Process queued responses (DSR, etc.)
        let responses = self.grid.take_responses();
        for response in responses {
            let _ = self.write_input(response.as_bytes());
        }

        // Check child exit
        if running {
            if let Ok(Some(exit_status)) = self.child.try_wait() {
                self.exit_code = Some(exit_status.exit_code() as i32);
                running = false;
            }
        }

        running
    }

    /// Write input to the PTY (send to shell)
    pub fn write_input(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(data)?;
        #[cfg(target_os = "windows")]
        self.writer.flush()?;
        Ok(())
    }

    /// Flush buffered input
    pub fn flush_input(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }

    /// Send pasted text respecting bracketed paste mode
    #[allow(dead_code)]
    pub fn send_paste(&mut self, text: &str) -> std::io::Result<()> {
        if self.grid.bracketed_paste_mode {
            self.write_input(b"\x1b[200~")?;
            self.write_input(text.as_bytes())?;
            self.write_input(b"\x1b[201~")?;
        } else {
            self.write_input(text.as_bytes())?;
        }
        self.writer.flush()
    }

    /// Check if child process is still alive
    #[allow(dead_code)]
    pub fn is_alive(&mut self) -> bool {
        if self.exit_code.is_some() {
            return false;
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.exit_code = Some(status.exit_code() as i32);
                false
            }
            _ => true,
        }
    }

    /// Get child process PID
    #[allow(dead_code)]
    pub fn get_pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// Get the name of the foreground process (macOS)
    #[cfg(target_os = "macos")]
    pub fn get_foreground_process_name(&self) -> Option<String> {
        let child_pid = self.child.process_id()?;

        let output = Command::new("ps")
            .args(["-o", "tpgid=", "-p", &child_pid.to_string()])
            .output()
            .ok()?;

        let tpgid = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .ok()?;

        let output = Command::new("ps")
            .args(["-o", "comm=", "-p", &tpgid.to_string()])
            .output()
            .ok()?;

        let process_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if process_name.is_empty() {
            let output = Command::new("ps")
                .args(["-o", "comm=", "-p", &child_pid.to_string()])
                .output()
                .ok()?;
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if name.is_empty() {
                None
            } else {
                Some(name.rsplit('/').next().unwrap_or(&name).to_string())
            }
        } else {
            Some(
                process_name
                    .rsplit('/')
                    .next()
                    .unwrap_or(&process_name)
                    .to_string(),
            )
        }
    }

    /// Get the name of the foreground process (Linux)
    #[cfg(target_os = "linux")]
    pub fn get_foreground_process_name(&self) -> Option<String> {
        use std::fs;

        let child_pid = self.child.process_id()?;
        let stat_path = format!("/proc/{}/stat", child_pid);
        let stat_content = fs::read_to_string(&stat_path).ok()?;

        let comm_end = stat_content.rfind(')')?;
        let after_comm = &stat_content[comm_end + 2..];
        let parts: Vec<&str> = after_comm.split_whitespace().collect();

        if parts.len() < 6 {
            return None;
        }

        let tpgid: u32 = parts[5].parse().ok()?;

        let comm_path = format!("/proc/{}/comm", tpgid);
        let name = fs::read_to_string(&comm_path)
            .ok()
            .or_else(|| fs::read_to_string(format!("/proc/{}/comm", child_pid)).ok())?
            .trim()
            .to_string();

        if name.is_empty() { None } else { Some(name) }
    }

    /// Get the name of the foreground process (Windows fallback)
    #[cfg(target_os = "windows")]
    pub fn get_foreground_process_name(&self) -> Option<String> {
        let child_pid = self.child.process_id()?;

        let output = Command::new("wmic")
            .args([
                "process",
                "where",
                &format!("ProcessId={}", child_pid),
                "get",
                "Name",
                "/value",
            ])
            .output()
            .ok()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if let Some(name) = line.strip_prefix("Name=") {
                let name = name.trim();
                if !name.is_empty() {
                    let name = name.strip_suffix(".exe").unwrap_or(name);
                    return Some(name.to_string());
                }
            }
        }
        None
    }
}
