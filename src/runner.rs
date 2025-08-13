use crate::ctrlc;
use crate::error::{DryRun, SkipStep};
use crate::execution_context::ExecutionContext;
use crate::report::{Report, StepResult};
use crate::step::Step;
use crate::terminal::print_error;
use crate::terminal::should_retry;
use color_eyre::eyre::Result;
use std::borrow::Cow;
use std::fmt::Debug;
use tracing::debug;

pub struct Runner<'a> {
    ctx: &'a ExecutionContext<'a>,
    report: Report<'a>,
}

impl<'a> Runner<'a> {
    pub fn new(ctx: &'a ExecutionContext) -> Runner<'a> {
        Runner {
            ctx,
            report: Report::new(),
        }
    }

    pub fn execute<F, M>(&mut self, step: Step, key: M, func: F) -> Result<()>
    where
        F: Fn() -> Result<()>,
        M: Into<Cow<'a, str>> + Debug,
    {
        if !self.ctx.config().should_run(step) {
            return Ok(());
        }

        let key = key.into();
        debug!("Step {:?}", key);

        // alter the `func` to put it in a span
        let func = || {
            let span =
                tracing::span!(parent: tracing::Span::none(), tracing::Level::TRACE, "step", step = ?step, key = %key);
            let _guard = span.enter();
            func()
        };

        loop {
            match func() {
                Ok(()) => {
                    self.report.push_result(Some((key, StepResult::Success)));
                    break;
                }
                Err(e) if e.downcast_ref::<DryRun>().is_some() => break,
                Err(e) if e.downcast_ref::<SkipStep>().is_some() => {
                    if self.ctx.config().verbose() || self.ctx.config().show_skipped() {
                        self.report.push_result(Some((key, StepResult::Skipped(e.to_string()))));
                    }
                    break;
                }
                Err(e) => {
                    debug!("Step {:?} failed: {:?}", key, e);
                    let interrupted = ctrlc::interrupted();
                    if interrupted {
                        ctrlc::unset_interrupted();
                    }

                    let ignore_failure = self.ctx.config().ignore_failure(step);
                    let should_ask = interrupted || !(self.ctx.config().no_retry() || ignore_failure);
                    let should_retry = if should_ask {
                        print_error(&key, format!("{e:?}"));
                        should_retry(interrupted, key.as_ref())?
                    } else {
                        false
                    };

                    if !should_retry {
                        self.report.push_result(Some((
                            key,
                            if ignore_failure {
                                StepResult::Ignored
                            } else {
                                StepResult::Failure
                            },
                        )));
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn report(&self) -> &Report<'_> {
        &self.report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CommandLineArgs, Config};
    use crate::execution_context::{ExecutionContext, RunType};
    use crate::sudo::Sudo;
    use clap::Parser;

    // Build a minimal context with dry-run to avoid side effects
    fn make_ctx() -> ExecutionContext<'static> {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let cfg_path = tmp.path().join("nonexistent.toml");
        let opt = CommandLineArgs::parse_from([
            "topgrade",
            "--dry-run",
            "--skip-notify",
            "--no-retry",
            "--config",
            cfg_path.to_str().unwrap(),
        ]);
        let config = Config::load(opt).expect("config load");
        let sudo = config.sudo_command().map_or_else(Sudo::detect, Sudo::new);
        #[cfg(target_os = "linux")]
        let distribution = Box::leak(Box::new(crate::steps::linux::Distribution::detect()));
        ExecutionContext::new(
            RunType::new(true),
            sudo,
            Box::leak(Box::new(config)),
            #[cfg(target_os = "linux")]
            distribution,
        )
    }

    #[test]
    fn runner_records_success_and_failure() {
        let ctx = make_ctx();
        let mut runner = Runner::new(&ctx);

        runner
            .execute(crate::step::Step::Bin, "ok", || Ok(()))
            .expect("execute ok");
        runner
            .execute(crate::step::Step::Bin, "fail", || Err(color_eyre::eyre::eyre!("boom")))
            .expect("execute fail handled");

        let mut seen_ok = false;
        let mut seen_fail = false;
        for (k, v) in runner.report().data() {
            if k == "ok" {
                assert!(matches!(v, StepResult::Success));
                seen_ok = true;
            }
            if k == "fail" {
                assert!(matches!(v, StepResult::Failure | StepResult::Ignored));
                seen_fail = true;
            }
        }
        assert!(seen_ok);
        assert!(seen_fail);
    }
}
