use anyhow::Result;
use clap::{Parser, Subcommand};
use nl_host::{audio, core, network::ControlClient};

#[derive(Parser, Debug)]
#[command(author, version, about = "NL-Mirror: High-speed Android mirroring")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(short, long, default_value_t = 8888)]
    port: u16,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Mirror {
        #[arg(long, default_value_t = 8000000)]
        bitrate: u32,

        #[arg(long, default_value_t = 1080)]
        max_size: u32,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Turn screen off while mirroring
        #[arg(long)]
        turn_screen_off: bool,

        /// Enable audio streaming (Android 11+ required)
        #[arg(long, default_value_t = true)]
        audio: bool,

        /// Disable audio streaming
        #[arg(long)]
        no_audio: bool,
    },
    Tap {
        x: f32,
        y: f32,
    },
    Stats,
    Hierarchy,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    match args.command.unwrap_or(Commands::Mirror {
        bitrate: 8000000,
        max_size: 1080,
        verbose: false,
        turn_screen_off: false,
        audio: true,
        no_audio: false,
    }) {
        Commands::Tap { x, y } => {
            let mut client = ControlClient::connect(&args.host, args.port + 1)?;
            client.tap(x, y)?;
            println!("Tap sent to ({}, {})", x, y);
        }
        Commands::Stats => {
            let mut client = ControlClient::connect(&args.host, args.port + 1)?;
            println!("{}", client.get_stats()?);
        }
        Commands::Hierarchy => {
            let mut client = ControlClient::connect(&args.host, args.port + 1)?;
            println!("{}", client.get_hierarchy()?);
        }
        Commands::Mirror {
            bitrate,
            max_size,
            verbose,
            turn_screen_off,
            audio: enable_audio,
            no_audio,
        } => {
            // Apply verbose config
            core::VERBOSE.store(verbose, std::sync::atomic::Ordering::SeqCst);

            // Start audio pipeline if enabled
            let audio_enabled = enable_audio && !no_audio;
            if audio_enabled {
                audio::start_audio_pipeline(args.host.clone(), args.port + 2);
            }

            core::run(args.host, args.port, bitrate, max_size, turn_screen_off)?;
        }
    }
    Ok(())
}
