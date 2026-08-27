use chainctl_core::ChainctlError;

use super::Context;

pub fn run(_ctx: &Context) -> Result<(), ChainctlError> {
    println!("chainctl {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
