use std::path::PathBuf;

use argh::FromArgs;

#[derive(FromArgs)]
#[argh(description = "Alpen Vertex sequencer")]
pub struct Args {
    #[argh(
        option,
        short = 'd',
        description = "datadir path that will contain databases"
    )]
    pub datadir: PathBuf,

    #[argh(option, short = 'r', description = "JSON-RPC port")]
    pub rpc_port: u16,

    #[argh(option, description = "bitcoind RPC host")]
    pub bitcoind_host: String,

    #[argh(option, description = "bitcoind RPC user")]
    pub bitcoind_user: String,

    #[argh(option, description = "bitcoind RPC password")]
    pub bitcoind_password: String,

    #[argh(
        option,
        short = 'n',
        description = "L1 network to run on",
        default = "\"regtest\".to_owned()"
    )]
    pub network: String,
}
