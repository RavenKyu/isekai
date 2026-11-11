use clap::{Parser, Subcommand};
#[derive(Parser, Debug)]
#[command(name = "isekai", version)]
pub struct Cli {
    #[arg(long,value_parser=clap::value_parser!(u32).range(1..),global=true)]
    pub smoke_frames: Option<u32>,
    #[command(subcommand)]
    pub command: Option<Command>,
}
#[derive(Subcommand, Debug)]
pub enum Command {
    Play {
        source: String,
        animation: Option<String>,
    },
}
impl Cli {
    pub fn smoke(&self) -> Result<Option<crate::smoke::Smoke>, String> {
        let expect = if cfg!(feature = "dev") {
            crate::smoke::Expect::Drawn
        } else if self.command.is_some() {
            crate::smoke::Expect::Sprite
        } else {
            crate::smoke::Expect::Blank
        };
        crate::smoke::Smoke::parse(
            self.smoke_frames
                .map(|n| vec!["--smoke-frames".into(), n.to_string()])
                .unwrap_or_default(),
            expect,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grammar() {
        for args in [
            vec!["isekai", "play"],
            vec!["isekai", "--play", "x"],
            vec!["isekai", "play", "x", "a", "extra"],
            vec!["isekai", "--unknown"],
            vec!["isekai", "--smoke-frames", "3", "--smoke-frames", "3"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
        for bad in ["0", "-1", "4294967296", "abc"] {
            assert!(Cli::try_parse_from(["isekai", "--smoke-frames", bad]).is_err());
        }
        assert!(
            Cli::try_parse_from(["isekai", "play", "x", "cycle", "--smoke-frames", "3"]).is_ok()
        );
    }
}
