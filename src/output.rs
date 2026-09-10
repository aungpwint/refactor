use console::Style;
use serde::Serialize;
use std::io::Write;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Output {
    pub verbose: bool,
    pub quiet: bool,
    pub json_mode: bool,
    pub color: bool,
    sink: Sink,
}

#[derive(Clone, Default)]
pub struct Capture(Arc<Mutex<String>>);

impl Capture {
    pub fn take(&self) -> String {
        self.0
            .lock()
            .map(|mut guard| std::mem::take(&mut *guard))
            .unwrap_or_default()
    }
}

#[derive(Clone)]
enum Sink {
    Stdio,
    Capture(Arc<Mutex<String>>),
}

#[derive(Serialize)]
struct JsonOutput {
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files_scanned: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files_changed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacements: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl Output {
    pub fn new(verbose: bool, quiet: bool, json_mode: bool, no_color: bool) -> Self {
        Self {
            verbose,
            quiet,
            json_mode,
            color: !no_color,
            sink: Sink::Stdio,
        }
    }

    pub fn captured(verbose: bool, quiet: bool, json_mode: bool) -> (Self, Capture) {
        let capture = Capture::default();
        let output = Self {
            verbose,
            quiet,
            json_mode,
            color: false,
            sink: Sink::Capture(capture.0.clone()),
        };
        (output, capture)
    }

    fn write_out(&self, text: &str) {
        match &self.sink {
            Sink::Stdio => println!("{text}"),
            Sink::Capture(buffer) => {
                if let Ok(mut guard) = buffer.lock() {
                    guard.push_str(text);
                    guard.push('\n');
                }
            }
        }
    }

    fn write_err(&self, text: &str) {
        match &self.sink {
            Sink::Stdio => eprintln!("{text}"),
            Sink::Capture(buffer) => {
                if let Ok(mut guard) = buffer.lock() {
                    guard.push_str(text);
                    guard.push('\n');
                }
            }
        }
    }

    pub fn success(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            self.print_json("ok", Some(msg.to_string()), None);
        } else if self.color {
            let style = Style::new().green().bold();
            self.write_out(&format!("  {} {}", style.apply_to("✓"), msg));
        } else {
            self.write_out(&format!("  ✓ {msg}"));
        }
    }

    pub fn info(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if !self.json_mode {
            self.write_out(&format!("  {msg}"));
        }
    }

    pub fn warn(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            self.print_json("warn", Some(msg.to_string()), None);
        } else if self.color {
            let style = Style::new().yellow().bold();
            self.write_out(&format!("  {} {}", style.apply_to("!"), msg));
        } else {
            self.write_out(&format!("  ! {msg}"));
        }
    }

    pub fn error(&self, msg: &str) {
        if self.json_mode {
            self.print_json("error", Some(msg.to_string()), None);
        } else if self.color {
            let style = Style::new().red().bold();
            self.write_err(&format!("  {} {}", style.apply_to("✗"), msg));
        } else {
            self.write_err(&format!("  ✗ {msg}"));
        }
    }

    pub fn verbose_msg(&self, msg: &str) {
        if self.verbose && !self.quiet && !self.json_mode {
            self.write_out(&format!("  [verbose] {msg}"));
        }
    }

    pub fn heading(&self, msg: &str) {
        if self.quiet || self.json_mode {
            return;
        }
        self.write_out("");
        if self.color {
            let style = Style::new().bold();
            self.write_out(&style.apply_to(msg).to_string());
        } else {
            self.write_out(msg);
        }
        self.write_out(&"─".repeat(msg.len().min(60)));
    }

    pub fn line(&self, msg: &str) {
        if self.quiet || self.json_mode {
            return;
        }
        self.write_out(&format!("  {msg}"));
    }

    pub fn blank(&self) {
        if !self.quiet && !self.json_mode {
            self.write_out("");
        }
    }

    pub fn progress(&self, msg: &str) {
        if matches!(self.sink, Sink::Capture(_)) {
            return;
        }
        if self.quiet || self.json_mode {
            return;
        }
        print!("\r  {msg}...");
        std::io::stdout().flush().ok();
    }

    pub fn progress_done(&self) {
        if matches!(self.sink, Sink::Capture(_)) {
            return;
        }
        if self.quiet || self.json_mode {
            return;
        }
        print!("\r\x1B[K");
        std::io::stdout().flush().ok();
    }

    pub fn print_json(
        &self,
        command: &str,
        message: Option<String>,
        data: Option<serde_json::Value>,
    ) {
        let output = JsonOutput {
            command: command.to_string(),
            message,
            files_scanned: None,
            files_changed: None,
            replacements: None,
            errors: None,
            data,
        };
        if let Ok(json) = serde_json::to_string_pretty(&output) {
            self.write_out(&json);
        }
    }

    pub fn print_json_result(&self, result: &ScanResult) {
        if self.json_mode {
            let json = serde_json::to_string_pretty(result).unwrap_or_default();
            self.write_out(&json);
        }
    }
}

#[derive(Serialize)]
pub struct ScanResult {
    pub command: String,
    pub files_scanned: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_changed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacements: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
