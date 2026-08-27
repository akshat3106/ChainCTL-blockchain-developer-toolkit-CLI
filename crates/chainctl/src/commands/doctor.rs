use chainctl_core::ChainctlError;
use owo_colors::OwoColorize;

use super::Context;

struct Check {
    label: &'static str,
    ok: bool,
    detail: String,
}

pub fn run(ctx: &Context) -> Result<(), ChainctlError> {
    let mut checks = Vec::new();

    let paths_ok = ctx.paths.ensure_dirs().is_ok();
    checks.push(Check {
        label: "Config directory",
        ok: paths_ok,
        detail: ctx.paths.root.display().to_string(),
    });

    match chainctl_registry::load_embedded() {
        Ok(registry) => {
            let faucet_count: usize = registry.chains.iter().map(|c| c.faucets.len()).sum();
            checks.push(Check {
                label: "Embedded registry",
                ok: true,
                detail: format!("{} chains, {faucet_count} faucets", registry.chains.len()),
            });
        }
        Err(e) => checks.push(Check {
            label: "Embedded registry",
            ok: false,
            detail: e.to_string(),
        }),
    }

    if ctx.paths.registry_file.exists() {
        match chainctl_registry::load(Some(&ctx.paths.registry_file)) {
            Ok(registry) => checks.push(Check {
                label: "Local registry.json",
                ok: true,
                detail: format!("{} chains", registry.chains.len()),
            }),
            Err(e) => checks.push(Check {
                label: "Local registry.json",
                ok: false,
                detail: e.to_string(),
            }),
        }
    } else {
        checks.push(Check {
            label: "Local registry.json",
            ok: true,
            detail: "not present yet — run `chainctl update`".to_string(),
        });
    }

    let mut all_ok = true;
    for check in &checks {
        all_ok &= check.ok;
        let icon = if check.ok {
            if ctx.theme.color { "✓".green().to_string() } else { "ok".to_string() }
        } else if ctx.theme.color {
            "✗".red().to_string()
        } else {
            "FAIL".to_string()
        };
        println!("{icon} {:<22} {}", check.label, check.detail);
    }

    if !all_ok {
        return Err(ChainctlError::Config(
            "one or more environment checks failed".to_string(),
        ));
    }
    Ok(())
}
