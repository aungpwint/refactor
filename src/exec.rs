#[derive(Debug, Clone, Copy, Default)]
pub struct ExecOptions {
    pub dry_run: bool,
    pub confirm: bool,
}

impl ExecOptions {
    pub fn new(dry_run: bool, confirm: bool) -> Self {
        Self { dry_run, confirm }
    }
}
