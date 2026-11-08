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
    started_ms: Option<u64>,
    opened: bool,
    probe_received: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    pub frames: u32,
    pub vtx: usize,
    pub lit: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Continue,
    SendKeys,
    Complete(Report),
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
            started_ms: None,
            opened: false,
            probe_received: false,
        }))
    }

    // The caller observes the real input queue; this module knows no event type.
    pub fn observe_key_probe(&mut self) {
        self.probe_received = true;
    }

    pub fn is_last_frame(&self, visible: bool) -> bool {
        self.started_ms.is_some()
            && (matches!(self.expect, Expect::Blank) && self.probe_received || visible)
            && self.frames + 1 == self.target
    }

    pub fn finish_frame(
        &mut self,
        now_ms: u64,
        visible: bool,
        vtx: usize,
        lit: usize,
    ) -> Result<Step, String> {
        let Some(started_ms) = self.started_ms else {
            if visible || vtx != 0 {
                return Err("dev tools must start hidden with vtx=0".into());
            }
            // One initial blank frame always precedes the input probe. Even a
            // one-frame non-dev smoke must process the probe before completion.
            self.started_ms = Some(now_ms);
            return Ok(Step::SendKeys);
        };
        // In the browser a fast blank smoke could finish before CDP sends its
        // probe. Count blank frames only once that input reached the app too.
        if matches!(self.expect, Expect::Blank) && !self.probe_received {
            if now_ms.saturating_sub(started_ms) >= 5_000 {
                return Err("smoke key probe did not arrive within 5s".into());
            }
            return Ok(Step::Continue);
        }
        if matches!(self.expect, Expect::Drawn) {
            if !self.opened && now_ms.saturating_sub(started_ms) >= 5_000 {
                return Err("dev tools did not open within 5s".into());
            }
            if !visible {
                if self.opened {
                    return Err("dev tools closed before smoke completed".into());
                }
                if vtx != 0 {
                    return Err("hidden dev tools drew UI geometry".into());
                }
                return Ok(Step::Continue);
            }
            self.opened = true;
        }
        self.frames += 1;
        if self.frames != self.target {
            return Ok(Step::Continue);
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
        Ok(Step::Complete(Report {
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

    fn smoke(expect: Expect, frames: u32) -> Smoke {
        Smoke::parse(["--smoke-frames".into(), frames.to_string()], expect)
            .unwrap()
            .unwrap()
    }

    #[test]
    fn hidden_frames_precede_input_and_only_open_frames_are_counted() {
        let mut smoke = smoke(Expect::Drawn, 3);
        assert!(!smoke.is_last_frame(false));
        assert_eq!(
            smoke.finish_frame(100, false, 0, 0).unwrap(),
            Step::SendKeys
        );
        assert_eq!(
            smoke.finish_frame(200, false, 0, 0).unwrap(),
            Step::Continue
        );
        for _ in 0..2 {
            assert!(!smoke.is_last_frame(true));
            assert_eq!(
                smoke.finish_frame(300, true, 92, 0).unwrap(),
                Step::Continue
            );
        }
        assert!(smoke.is_last_frame(true));
        let Step::Complete(report) = smoke.finish_frame(400, true, 92, 120).unwrap() else {
            panic!("missing report");
        };
        assert_eq!(report.line(), "rn-smoke: ok frames=3 vtx=92 lit=120");
        assert_eq!(
            failure("bad\n context\tstate"),
            "rn-smoke: fail bad context state"
        );
    }

    #[test]
    fn visible_start_hidden_geometry_and_early_closing_fail() {
        for (visible, vtx) in [(true, 0), (true, 92), (false, 92)] {
            assert!(
                smoke(Expect::Drawn, 3)
                    .finish_frame(0, visible, vtx, 0)
                    .is_err()
            );
        }
        let mut hidden = smoke(Expect::Drawn, 3);
        hidden.finish_frame(0, false, 0, 0).unwrap();
        assert!(hidden.finish_frame(1, false, 92, 0).is_err());
        let mut closing = smoke(Expect::Drawn, 3);
        closing.finish_frame(0, false, 0, 0).unwrap();
        closing.finish_frame(1, true, 92, 0).unwrap();
        assert!(
            closing
                .finish_frame(2, false, 0, 0)
                .unwrap_err()
                .contains("closed")
        );
    }

    #[test]
    fn opening_deadline_is_five_seconds_and_does_not_limit_open_frames() {
        let mut late = smoke(Expect::Drawn, 1);
        late.finish_frame(100, false, 0, 0).unwrap();
        assert_eq!(
            late.finish_frame(5099, false, 0, 0).unwrap(),
            Step::Continue
        );
        assert_eq!(
            late.finish_frame(5100, false, 0, 0).unwrap_err(),
            "dev tools did not open within 5s"
        );
        let mut timely = smoke(Expect::Drawn, 2);
        timely.finish_frame(100, false, 0, 0).unwrap();
        timely.finish_frame(5099, true, 92, 0).unwrap();
        assert!(matches!(
            timely.finish_frame(20_000, true, 92, 120).unwrap(),
            Step::Complete(_)
        ));
    }

    #[test]
    fn blank_mode_processes_probe_before_counting_and_requires_a_black_frame() {
        for (vtx, lit) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let mut smoke = smoke(Expect::Blank, 1);
            assert_eq!(smoke.finish_frame(0, false, 0, 0).unwrap(), Step::SendKeys);
            assert!(!smoke.is_last_frame(false));
            assert_eq!(smoke.finish_frame(1, false, 0, 0).unwrap(), Step::Continue);
            smoke.observe_key_probe();
            assert!(smoke.is_last_frame(false));
            let result = smoke.finish_frame(1, false, vtx, lit);
            if vtx == 0 && lit == 0 {
                let Step::Complete(report) = result.unwrap() else {
                    panic!("missing report");
                };
                assert_eq!(report.line(), "rn-smoke: ok frames=1 vtx=0 lit=0");
            } else {
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn blank_mode_fails_if_the_probe_never_reaches_the_app() {
        let mut smoke = smoke(Expect::Blank, 1);
        smoke.finish_frame(100, false, 0, 0).unwrap();
        assert_eq!(
            smoke.finish_frame(5099, false, 0, 0).unwrap(),
            Step::Continue
        );
        assert!(
            smoke
                .finish_frame(5100, false, 0, 0)
                .unwrap_err()
                .contains("key probe")
        );
    }

    #[test]
    fn empty_geometry_or_clear_framebuffer_fails() {
        for (vtx, lit) in [(0, 9), (9, 0), (0, 0)] {
            let mut smoke = smoke(Expect::Drawn, 1);
            smoke.finish_frame(0, false, 0, 0).unwrap();
            assert!(smoke.finish_frame(1, true, vtx, lit).is_err());
        }
    }
}
