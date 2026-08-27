use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct AbiCmd {
    #[command(subcommand)]
    action: AbiAction,
}

#[derive(Subcommand)]
enum AbiAction {
    /// Encode a function call into calldata, e.g.
    /// `chainctl abi encode "transfer(address,uint256)" 0xabc... 1000`.
    Encode { signature: String, args: Vec<String> },
    /// Decode calldata against a function signature.
    Decode { signature: String, calldata: String },
}

pub fn run(_ctx: &Context, cmd: AbiCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        AbiAction::Encode { signature, args } => encode(&signature, &args),
        AbiAction::Decode { signature, calldata } => decode(&signature, &calldata),
    }
}

fn encode(signature: &str, args: &[String]) -> Result<(), ChainctlError> {
    let sig = chainctl_provider::abi::parse_signature(signature).map_err(ChainctlError::Config)?;
    if sig.inputs.len() != args.len() {
        return Err(ChainctlError::Config(format!(
            "'{signature}' takes {} argument(s), got {}",
            sig.inputs.len(),
            args.len()
        )));
    }

    let params = chainctl_provider::abi::encode_params(&sig.inputs, args).map_err(ChainctlError::Config)?;
    let mut calldata = sig.selector().to_vec();
    calldata.extend_from_slice(&params);

    println!("{}", chainctl_provider::abi::encode_hex(&calldata));
    Ok(())
}

fn decode(signature: &str, calldata: &str) -> Result<(), ChainctlError> {
    let sig = chainctl_provider::abi::parse_signature(signature).map_err(ChainctlError::Config)?;
    let bytes = chainctl_provider::abi::decode_hex(calldata).map_err(ChainctlError::Config)?;
    if bytes.len() < 4 {
        return Err(ChainctlError::Config("calldata is shorter than a 4-byte selector".to_string()));
    }

    let expected = sig.selector();
    if bytes[0..4] != expected {
        eprintln!(
            "warning: calldata selector {} doesn't match '{signature}' selector {}",
            chainctl_provider::abi::encode_hex(&bytes[0..4]),
            chainctl_provider::abi::encode_hex(&expected)
        );
    }

    let values = chainctl_provider::abi::decode_params(&sig.inputs, &bytes[4..]).map_err(ChainctlError::Config)?;
    for (t, v) in sig.inputs.iter().zip(values) {
        println!("{t}: {v}");
    }
    Ok(())
}
