mod analyzers;
mod cli;
mod commands;
mod config;
mod context;
mod error;
mod exec;
mod filesystem;
mod languages;
mod mcp;
mod output;
mod refactor;
mod scanner;
mod utils;

use std::process;

fn main() {
    let cli = cli::parse();

    if let cli::Command::Mcp(args) = &cli.command {
        let root = args.root.clone().unwrap_or_else(|| cli.global.root.clone());
        process::exit(mcp::run_server(root));
    }

    let output = output::Output::new(
        cli.global.verbose,
        cli.global.quiet,
        cli.global.json,
        cli.global.no_color,
    );

    let mut config = match config::load(&cli.global.root) {
        Ok(c) => c,
        Err(e) => {
            output.error(&format!("Configuration error: {e}"));
            process::exit(3);
        }
    };
    config::apply_cli_args(&mut config, &cli.global);

    let mut ctx = match context::RepoContext::new(&cli.global.root, &config) {
        Ok(c) => c,
        Err(e) => {
            output.error(&format!("Repository error: {e}"));
            process::exit(4);
        }
    };

    ctx.init_threads(cli.global.threads);

    let opts = exec::ExecOptions::new(cli.global.dry_run, cli.global.yes);

    let result = match cli.command {
        cli::Command::Scan(args) => commands::scan::run(&ctx, &output, &args),
        cli::Command::Check(args) => commands::check::run(&ctx, &output, &args),
        cli::Command::Replace(args) => commands::replace::run(&ctx, &output, &args, &opts),
        cli::Command::Rename(args) => commands::rename::run(&ctx, &output, &args, &opts),
        cli::Command::Imports(args) => commands::imports::run(&ctx, &output, &args, &opts),
        cli::Command::Paths(args) => commands::paths::run(&ctx, &output, &args, &opts),
        cli::Command::References(args) => commands::references::run(&ctx, &output, &args),
        cli::Command::Unused(args) => commands::unused::run(&ctx, &output, &args),
        cli::Command::Duplicates(args) => commands::duplicates::run(&ctx, &output, &args),
        cli::Command::Normalize(args) => commands::normalize::run(&ctx, &output, &args),
        cli::Command::Migrate(args) => commands::migrate::run(&ctx, &output, &args, &opts),
        cli::Command::Clean(args) => commands::clean::run(&ctx, &output, &args, &opts),
        cli::Command::Update => commands::update::run(&ctx, &output),
        cli::Command::Mcp(_) => unreachable!("mcp handled above"),
    };

    match result {
        Ok(code) => process::exit(code),
        Err(e) => {
            output.error(&format!("{e}"));
            process::exit(e.exit_code());
        }
    }
}
