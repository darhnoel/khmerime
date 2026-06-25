use std::sync::Arc;

use super::SpanProposalMode;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct SpanProposalRequest<'a> {
    pub full_input: &'a str,
    pub span: &'a str,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpanProposal {
    pub output: String,
    pub recovered_roman: String,
    pub confidence_bps: u16,
    pub source: &'static str,
}

pub(crate) trait SpanProposalProvider: Send + Sync {
    fn candidate_ends(&self, full_input: &str, start: usize, chars: &[char]) -> Vec<usize>;
    fn propose(&self, request: &SpanProposalRequest<'_>) -> Vec<SpanProposal>;
}

pub(crate) fn provider_for_mode(mode: SpanProposalMode) -> Option<Arc<dyn SpanProposalProvider>> {
    match mode {
        SpanProposalMode::Disabled => None,
        SpanProposalMode::StaticTest => Some(Arc::new(StaticTestSpanProposalProvider)),
        #[cfg(feature = "byt5")]
        SpanProposalMode::Byt5 => super::byt5_provider::Byt5SpanProposalProvider::load_from_env(),
        #[cfg(not(feature = "byt5"))]
        SpanProposalMode::Byt5 => None,
    }
}

struct StaticTestSpanProposalProvider;

impl StaticTestSpanProposalProvider {
    fn rows() -> &'static [(&'static str, &'static str)] {
        &[("knhm", "ខ្ញុំ"), ("ttv", "ទៅ"), ("salarien", "សាលារៀន")]
    }
}

impl SpanProposalProvider for StaticTestSpanProposalProvider {
    fn candidate_ends(&self, full_input: &str, start: usize, chars: &[char]) -> Vec<usize> {
        let tail = chars[start..].iter().collect::<String>();
        Self::rows()
            .iter()
            .filter_map(|(roman, _)| {
                tail.starts_with(roman)
                    .then_some(start + roman.chars().count())
                    .filter(|end| *end <= full_input.chars().count())
            })
            .collect()
    }

    fn propose(&self, request: &SpanProposalRequest<'_>) -> Vec<SpanProposal> {
        Self::rows()
            .iter()
            .filter_map(|(roman, output)| {
                (*roman == request.span).then(|| SpanProposal {
                    output: (*output).to_owned(),
                    recovered_roman: (*roman).to_owned(),
                    confidence_bps: 9_500,
                    source: "static-test",
                })
            })
            .collect()
    }
}
