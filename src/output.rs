use console::Style;
use serde::Serialize;
use std::io::Write;

#[derive(Clone)]
pub struct Output {
    pub verbose: bool,
    pub quiet: bool,
    pub json_mode: bool,
    pub color: bool,
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
            println!("  {} {}", style.apply_to("✓"), msg);
        } else {
            println!("  ✓ {msg}");
        }
    }

    pub fn info(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if !self.json_mode {
            println!("  {msg}");
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
            println!("  {} {}", style.apply_to("!"), msg);
        } else {
            println!("  ! {msg}");
        }
    }

    pub fn error(&self, msg: &str) {
        if self.json_mode {
            self.print_json("error", Some(msg.to_string()), None);
        } else if self.color {
            let style = Style::new().red().bold();
            eprintln!("  {} {}", style.apply_to("✗"), msg);
        } else {
            eprintln!("  ✗ {msg}");
        }
    }

    pub fn verbose_msg(&self, msg: &str) {
        if self.verbose && !self.quiet && !self.json_mode {
            println!("  [verbose] {msg}");
        }
    }

    pub fn heading(&self, msg: &str) {
        if self.quiet || self.json_mode {
            return;
        }
        println!();
        if self.color {
            let style = Style::new().bold();
            println!("{}", style.apply_to(msg));
        } else {
            println!("{msg}");
        }
        println!("{}", "─".repeat(msg.len().min(60)));
    }

    pub fn line(&self, msg: &str) {
        if self.quiet || self.json_mode {
            return;
        }
        println!("  {msg}");
    }

    pub fn blank(&self) {
        if !self.quiet && !self.json_mode {
            println!();
        }
    }

    pub fn progress(&self, msg: &str) {
        if self.quiet || self.json_mode {
            return;
        }
        print!("\r  {msg}...");
        std::io::stdout().flush().ok();
    }

    pub fn progress_done(&self) {
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
            println!("{json}");
        }
    }

    pub fn print_json_result(&self, result: &ScanResult) {
        if self.json_mode {
            let json = serde_json::to_string_pretty(result).unwrap_or_default();
            println!("{json}");
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
