//! Platform-independent smoke arguments, completion and output contract.

#[derive(Debug, Clone, Copy)]
pub enum Expect {
    Drawn,
    Blank,
}

#[derive(Debug)]
pub struct Smoke {
    expect: Expect,
    target: u32,
    frames: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    pub frames: u32,
    pub vtx: usize,
    pub lit: usize,
}

impl Report {
    pub fn line(&self) -> String {
        format!(
            "rn-smoke: ok frames={} vtx={} lit={}",
            self.frames, self.vtx, self.lit
        )
    }
}

pub fn failure(reason: &str) -> String {
    // Keep even multiline backend diagnostics inside the one-line contract.
    format!(
        "rn-smoke: fail {}",
        reason.split_whitespace().collect::<Vec<_>>().join(" ")
    )
}

impl Smoke {
    pub fn parse(
        args: impl IntoIterator<Item = String>,
        expect: Expect,
    ) -> Result<Option<Self>, String> {
        let mut args = args.into_iter();
        let Some(arg) = args.next() else {
            return Ok(None);
        };
        if arg != "--smoke-frames" {
            return Err(format!("unknown argument: {arg}"));
        }
        let value = args
            .next()
            .ok_or("--smoke-frames requires a positive integer")?;
        if args.next().is_some() {
            return Err("unexpected arguments after --smoke-frames".into());
        }
        let target = value
            .parse::<u32>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("--smoke-frames requires a positive integer")?;
        Ok(Some(Self {
            expect,
            target,
            frames: 0,
        }))
    }

    pub fn is_last_frame(&self) -> bool {
        self.frames + 1 == self.target
    }

    pub fn finish_frame(&mut self, vtx: usize, lit: usize) -> Result<Option<Report>, String> {
        self.frames += 1;
        if self.frames != self.target {
            return Ok(None);
        }
        match self.expect {
            Expect::Drawn if vtx == 0 => return Err("vtx=0: no UI geometry".into()),
            Expect::Drawn if lit == 0 => {
                return Err("lit=0: framebuffer matches clear color".into());
            }
            Expect::Blank if vtx != 0 || lit != 0 => {
                return Err(format!("vtx={vtx} lit={lit}: non-dev frame is not blank"));
            }
            _ => {}
        }
        Ok(Some(Report {
            frames: self.frames,
            vtx,
            lit,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Smoke>, String> {
        Smoke::parse(args.iter().map(|s| s.to_string()), Expect::Drawn)
    }

    #[test]
    fn arguments_require_an_explicit_positive_frame_count() {
        assert!(parse(&[]).unwrap().is_none());
        for args in [
            vec!["--smoke-frames"],
            vec!["--smoke-frames", "0"],
            vec!["--smoke-frames", "-1"],
            vec!["--smoke-frames", "4294967296"],
            vec!["--smoke-frames", "abc"],
            vec!["--smoke-frames", "3", "extra"],
            vec!["--unknown"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn only_the_last_frame_is_judged_and_output_is_exact() {
        let mut smoke = parse(&["--smoke-frames", "3"]).unwrap().unwrap();
        for _ in 0..2 {
            assert!(!smoke.is_last_frame());
            assert_eq!(smoke.finish_frame(0, 0).unwrap(), None);
        }
        assert!(smoke.is_last_frame());
        let report = smoke.finish_frame(92, 120).unwrap().unwrap();
        assert_eq!(report.line(), "rn-smoke: ok frames=3 vtx=92 lit=120");
        assert_eq!(
            failure("bad\n context\tstate"),
            "rn-smoke: fail bad context state"
        );
    }

    #[test]
    fn blank_mode_requires_zero_geometry_and_black_pixels() {
        for (vtx, lit) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let mut smoke = Smoke::parse(["--smoke-frames".into(), "1".into()], Expect::Blank)
                .unwrap()
                .unwrap();
            let result = smoke.finish_frame(vtx, lit);
            if vtx == 0 && lit == 0 {
                assert_eq!(
                    result.unwrap().unwrap().line(),
                    "rn-smoke: ok frames=1 vtx=0 lit=0"
                );
            } else {
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn empty_geometry_or_clear_framebuffer_fails() {
        for (vtx, lit) in [(0, 9), (9, 0), (0, 0)] {
            let mut smoke = parse(&["--smoke-frames", "1"]).unwrap().unwrap();
            assert!(smoke.finish_frame(vtx, lit).is_err());
        }
    }
}
