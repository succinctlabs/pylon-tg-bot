use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[clap(long, env)]
    pub pylon_api_token: String,

    #[clap(long, env)]
    pub settings_path: Option<String>,
}
