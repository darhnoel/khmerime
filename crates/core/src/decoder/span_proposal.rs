use std::sync::{Arc, OnceLock};

use super::SpanProposalMode;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SpanProposalRequest<'a> {
    pub full_input: &'a str,
    pub span: &'a str,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanProposal {
    pub output: String,
    pub recovered_roman: String,
    pub confidence_bps: u16,
    pub source: &'static str,
}

pub trait SpanProposalProvider: Send + Sync {
    fn candidate_ends(&self, full_input: &str, start: usize, chars: &[char]) -> Vec<usize>;
    fn propose(&self, request: &SpanProposalRequest<'_>) -> Vec<SpanProposal>;
}

/// Injection point for a span-proposal provider supplied by a separate build (implemented
/// against `SpanProposalProvider`, registered once at startup). Absent = `Model` mode is inert,
/// so the public engine never carries a provider unless one is deliberately plugged in.
static REGISTERED_PROVIDER: OnceLock<Arc<dyn SpanProposalProvider>> = OnceLock::new();

/// Register the provider used for `SpanProposalMode::Model`. First registration wins.
pub fn register_span_proposal_provider(provider: Arc<dyn SpanProposalProvider>) {
    let _ = REGISTERED_PROVIDER.set(provider);
}

pub(crate) fn provider_for_mode(mode: SpanProposalMode) -> Option<Arc<dyn SpanProposalProvider>> {
    match mode {
        SpanProposalMode::Disabled => None,
        SpanProposalMode::StaticTest => Some(Arc::new(StaticTestSpanProposalProvider)),
        SpanProposalMode::Model => REGISTERED_PROVIDER.get().cloned(),
    }
}

/// Max roman span length a model provider is asked to transliterate — covers the longest
/// single Lexicon words (salarien=8, vinichchhai=11). Longer would blow up the query count;
/// the decoder's `over_budget` bail caps the rest.
#[allow(dead_code)] // used by the model-provider seam (feature-gated) + tests
const MAX_MODEL_SPAN_LEN: usize = 12;

/// Span ends a model provider offers from `start`: every length 2..=MAX that still fits in
/// `n` chars. Lets the lattice assemble multi-word segmentations (knhm|ttv|salarien) instead
/// of one whole-input span.
#[allow(dead_code)]
pub fn candidate_span_ends(start: usize, n: usize) -> Vec<usize> {
    ((start + 2)..=(start + MAX_MODEL_SPAN_LEN).min(n)).collect()
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

#[cfg(test)]
mod tests {
    use super::{candidate_span_ends, provider_for_mode, SpanProposalMode};

    // Inert-by-default invariant: no provider unless one is configured/compiled in, so
    // dev-default decoding keeps its existing no-provider path (one `is_some()` branch, no model).
    #[test]
    fn seam_is_inert_without_a_configured_provider() {
        assert!(provider_for_mode(SpanProposalMode::Disabled).is_none());
        assert!(
            provider_for_mode(SpanProposalMode::Model).is_none(),
            "Model must resolve to None until a provider is registered"
        );
    }

    #[test]
    fn candidate_span_ends_covers_subword_lengths() {
        // "knhmttvsalarien" (15 chars): from the start, offer spans of length 2..=12 so the
        // lattice can assemble knhm|ttv|salarien instead of one whole-input span.
        assert_eq!(candidate_span_ends(0, 15), (2..=12).collect::<Vec<usize>>());
    }

    #[test]
    fn candidate_span_ends_caps_and_truncates() {
        // mid-phrase (knhm+ttv = 7): salarien (len 8 -> end 15) is reachable
        assert_eq!(candidate_span_ends(7, 15), vec![9, 10, 11, 12, 13, 14, 15]);
        // near the end: only the spans that still fit
        assert_eq!(candidate_span_ends(13, 15), vec![15]);
        // no room for even a length-2 span -> empty
        assert!(candidate_span_ends(14, 15).is_empty());
        // long input: never exceed MAX span length from a start
        assert_eq!(*candidate_span_ends(0, 100).last().unwrap(), 12);
    }
}
