mod commands;

use clap::{CommandFactory, Parser};

/// git/kubectl-style plugin convention: an unrecognized first argument
/// (e.g. `chainctl foo`) is looked up as `chainctl-foo` on `$PATH` and, if
/// found, exec'd with the remaining args — lets the community ship modules
/// without forking core (ARCHITECTURE.md §14 Phase 5), with no dynamic
/// loading or plugin registry machinery needed.
fn try_dispatch_plugin(args: &[String]) -> Option<i32> {
    let first = args.get(1)?;
    if first.starts_with('-') {
        return None;
    }

    let known: Vec<String> = commands::Cli::command()
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    if known.iter().any(|name| name == first) || first == "help" {
        return None;
    }

    let plugin_path = find_on_path(&format!("chainctl-{first}"))?;
    let status = std::process::Command::new(plugin_path)
        .args(&args[2..])
        .status()
        .ok()?;
    Some(status.code().unwrap_or(1))
}

fn find_on_path(name: &str) -> Option<std::path::PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(format!("{name}{exe_suffix}")))
        .find(|candidate| candidate.is_file())
}

#[tokio::main]
async fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    if let Some(code) = try_dispatch_plugin(&raw_args) {
        std::process::exit(code);
    }

    let cli = commands::Cli::parse();
    // Rebuilt for error rendering even on early failure (e.g. Context::from_cli
    // itself fails) so error output stays consistently formatted.
    let no_color = cli.no_color;
    let output = cli.output.into();

    if let Err(err) = commands::dispatch(cli).await {
        let theme = chainctl_output::Theme::detect(no_color);
        eprintln!("{}", chainctl_output::render_error(&err, output, &theme));
        std::process::exit(err.exit_code());
    }
}
