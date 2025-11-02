use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "server", about = "Run the server with options")]
pub struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    pub proxy_host: String,
    #[arg(long, default_value_t = 8080)]
    pub proxy_port: u16,

    #[arg(long, default_value = "127.0.0.1")]
    pub forward_host: String,

    #[arg(long, default_value_t = 3000)]
    pub forward_port: u16,
}
