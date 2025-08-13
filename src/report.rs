use std::borrow::Cow;

pub enum StepResult {
    Success,
    Failure,
    Ignored,
    Skipped(String),
}

impl StepResult {
    pub fn failed(&self) -> bool {
        match self {
            StepResult::Success | StepResult::Ignored | StepResult::Skipped(_) => false,
            StepResult::Failure => true,
        }
    }
}

type CowString<'a> = Cow<'a, str>;
type ReportData<'a> = Vec<(CowString<'a>, StepResult)>;
pub struct Report<'a> {
    data: ReportData<'a>,
}

impl<'a> Report<'a> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn push_result<M>(&mut self, result: Option<(M, StepResult)>)
    where
        M: Into<CowString<'a>>,
    {
        if let Some((key, success)) = result {
            let key = key.into();

            debug_assert!(!self.data.iter().any(|(k, _)| k == &key), "{key} already reported");
            self.data.push((key, success));
        }
    }

    pub fn data(&self) -> &ReportData<'a> {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "already reported")]
    fn pushing_duplicate_key_panics_in_debug() {
        let mut report: Report = Report::new();
        report.push_result(Some(("k", StepResult::Success)));
        report.push_result(Some(("k", StepResult::Failure)));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn pushing_duplicate_key_allowed_in_release() {
        let mut report: Report = Report::new();
        report.push_result(Some(("k", StepResult::Success)));
        report.push_result(Some(("k", StepResult::Failure)));
        assert_eq!(report.data().len(), 2);
    }
}
