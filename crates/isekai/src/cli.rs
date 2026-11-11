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
/// Whether a failure must keep the smoke stdout contract. A parsed command line
/// knows; a rejected one is judged by the flag in either clap spelling.
#[cfg(not(target_arch = "wasm32"))]
pub fn smoke_requested(args: &[String], parsed: Option<&Cli>) -> bool {
    match parsed {
        Some(cli) => cli.smoke_frames.is_some(),
        None => args
            .iter()
            .any(|s| s == "--smoke-frames" || s.starts_with("--smoke-frames=")),
    }
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
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn smoke_contract_covers_both_flag_spellings() {
        let owned = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        for list in [
            &["isekai", "--smoke-frames", "3"][..],
            &["isekai", "--smoke-frames=3"],
            &["isekai", "play", "x", "--smoke-frames=3"],
        ] {
            let args = owned(list);
            let cli = Cli::try_parse_from(&args).unwrap();
            assert!(smoke_requested(&args, Some(&cli)), "{list:?}");
        }
        for list in [
            &["isekai", "--smoke-frames", "0"][..],
            &["isekai", "--smoke-frames=0"],
        ] {
            let args = owned(list);
            assert!(Cli::try_parse_from(&args).is_err(), "{list:?}");
            assert!(smoke_requested(&args, None), "{list:?}");
        }
        let args = owned(&["isekai", "play", "x"]);
        let cli = Cli::try_parse_from(&args).unwrap();
        assert!(!smoke_requested(&args, Some(&cli)));
    }
}
