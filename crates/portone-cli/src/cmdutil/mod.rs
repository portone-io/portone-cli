use clap::Args;

#[derive(Debug, Clone, Default, Args)]
pub struct AuthOpts {
    #[arg(
        long,
        value_name = "URL",
        help = "Base URL for API requests (default: https://api.portone.io)"
    )]
    pub base_url: Option<String>,

    #[arg(long, value_name = "NAME", help = "Configuration profile to use")]
    pub profile: Option<String>,
}
