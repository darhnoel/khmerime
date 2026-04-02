use std::fmt::Write;

use super::{DecodeFailure, DecodeResult, DecoderMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowMismatch {
    Top1Match,
    SameSetDifferentOrder,
    LegacyTopFoundInWfst,
    WfstTopFoundInLegacy,
    WfstEmpty,
    WfstFailed,
    WfstUnavailable,
    OutputMismatch,
}

impl ShadowMismatch {
    pub fn as_str(self) -> &'static str {
        match self {
            ShadowMismatch::Top1Match => "top1_match",
            ShadowMismatch::SameSetDifferentOrder => "same_set_different_order",
            ShadowMismatch::LegacyTopFoundInWfst => "legacy_top_found_in_wfst",
            ShadowMismatch::WfstTopFoundInLegacy => "wfst_top_found_in_legacy",
            ShadowMismatch::WfstEmpty => "wfst_empty",
            ShadowMismatch::WfstFailed => "wfst_failed",
            ShadowMismatch::WfstUnavailable => "wfst_unavailable",
            ShadowMismatch::OutputMismatch => "output_mismatch",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowObservation {
    pub mode: DecoderMode,
    pub input: String,
    pub mismatch: ShadowMismatch,
    pub legacy_latency_us: u64,
    pub wfst_latency_us: Option<u64>,
    pub legacy_failure: Option<String>,
    pub wfst_failure: Option<String>,
    pub legacy_top: Option<String>,
    pub wfst_top: Option<String>,
    pub legacy_top5: Vec<String>,
    pub wfst_top5: Vec<String>,
    pub legacy_top_in_wfst: bool,
    pub wfst_top_in_legacy: bool,
}

impl ShadowObservation {
    pub fn tsv_header() -> &'static str {
        "mode\tinput\tmismatch\tlegacy_latency_us\twfst_latency_us\tlegacy_failure\twfst_failure\tlegacy_top\twfst_top\tlegacy_top_in_wfst\twfst_top_in_legacy\tlegacy_top5\twfst_top5"
    }

    pub fn to_tsv_row(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            sanitize_field(&format!("{:?}", self.mode)),
            sanitize_field(&self.input),
            self.mismatch.as_str(),
            self.legacy_latency_us,
            self.wfst_latency_us
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            sanitize_optional(&self.legacy_failure),
            sanitize_optional(&self.wfst_failure),
            sanitize_optional(&self.legacy_top),
            sanitize_optional(&self.wfst_top),
            if self.legacy_top_in_wfst { "1" } else { "0" },
            if self.wfst_top_in_legacy { "1" } else { "0" },
            sanitize_field(&self.legacy_top5.join("|")),
            sanitize_field(&self.wfst_top5.join("|")),
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShadowSummary {
    pub total: usize,
    pub top1_match: usize,
    pub same_set_different_order: usize,
    pub legacy_top_found_in_wfst: usize,
    pub wfst_top_found_in_legacy: usize,
    pub wfst_empty: usize,
    pub wfst_failed: usize,
    pub wfst_unavailable: usize,
    pub output_mismatch: usize,
}

impl ShadowSummary {
    pub fn record(&mut self, observation: &ShadowObservation) {
        self.total += 1;
        match observation.mismatch {
            ShadowMismatch::Top1Match => self.top1_match += 1,
            ShadowMismatch::SameSetDifferentOrder => self.same_set_different_order += 1,
            ShadowMismatch::LegacyTopFoundInWfst => self.legacy_top_found_in_wfst += 1,
            ShadowMismatch::WfstTopFoundInLegacy => self.wfst_top_found_in_legacy += 1,
            ShadowMismatch::WfstEmpty => self.wfst_empty += 1,
            ShadowMismatch::WfstFailed => self.wfst_failed += 1,
            ShadowMismatch::WfstUnavailable => self.wfst_unavailable += 1,
            ShadowMismatch::OutputMismatch => self.output_mismatch += 1,
        }
    }

    pub fn format_report(&self) -> String {
        let mut report = String::new();
        let _ = writeln!(&mut report, "summary.total={}", self.total);
        let _ = writeln!(&mut report, "summary.top1_match={}", self.top1_match);
        let _ = writeln!(
            &mut report,
            "summary.same_set_different_order={}",
            self.same_set_different_order
        );
        let _ = writeln!(
            &mut report,
            "summary.legacy_top_found_in_wfst={}",
            self.legacy_top_found_in_wfst
        );
        let _ = writeln!(
            &mut report,
            "summary.wfst_top_found_in_legacy={}",
            self.wfst_top_found_in_legacy
        );
        let _ = writeln!(&mut report, "summary.wfst_empty={}", self.wfst_empty);
        let _ = writeln!(&mut report, "summary.wfst_failed={}", self.wfst_failed);
        let _ = writeln!(&mut report, "summary.wfst_unavailable={}", self.wfst_unavailable);
        let _ = writeln!(&mut report, "summary.output_mismatch={}", self.output_mismatch);
        report
    }
}

pub(crate) struct ShadowReport<'a> {
    pub observation: &'a ShadowObservation,
}

impl<'a> ShadowReport<'a> {
    pub(crate) fn format(&self) -> String {
        self.observation.to_tsv_row()
    }
}

pub(crate) fn build_shadow_observation(
    mode: DecoderMode,
    input: &str,
    legacy: &DecodeResult,
    wfst: Option<&DecodeResult>,
) -> ShadowObservation {
    let mismatch = categorize_mismatch(legacy, wfst);
    let legacy_top5 = top_candidates(legacy);
    let wfst_top5 = wfst.map(top_candidates).unwrap_or_default();
    let legacy_top = legacy_top5.first().cloned();
    let wfst_top = wfst_top5.first().cloned();
    let legacy_top_in_wfst = legacy_top
        .as_ref()
        .map(|top| wfst_top5.iter().any(|candidate| candidate == top))
        .unwrap_or(false);
    let wfst_top_in_legacy = wfst_top
        .as_ref()
        .map(|top| legacy_top5.iter().any(|candidate| candidate == top))
        .unwrap_or(false);

    ShadowObservation {
        mode,
        input: input.to_owned(),
        mismatch,
        legacy_latency_us: legacy.latency_us,
        wfst_latency_us: wfst.map(|result| result.latency_us),
        legacy_failure: legacy.failure.as_ref().map(format_failure),
        wfst_failure: wfst.and_then(|result| result.failure.as_ref().map(format_failure)),
        legacy_top,
        wfst_top,
        legacy_top5,
        wfst_top5,
        legacy_top_in_wfst,
        wfst_top_in_legacy,
    }
}

fn top_candidates(result: &DecodeResult) -> Vec<String> {
    result
        .candidates
        .iter()
        .take(5)
        .map(|candidate| candidate.text.clone())
        .collect()
}

fn categorize_mismatch(legacy: &DecodeResult, wfst: Option<&DecodeResult>) -> ShadowMismatch {
    let Some(wfst) = wfst else {
        return ShadowMismatch::WfstUnavailable;
    };
    if wfst.failure.is_some() {
        return ShadowMismatch::WfstFailed;
    }

    let legacy_top = legacy.candidates.first().map(|candidate| candidate.text.as_str());
    let wfst_top = wfst.candidates.first().map(|candidate| candidate.text.as_str());
    if legacy_top == wfst_top {
        return ShadowMismatch::Top1Match;
    }

    let legacy_set = legacy
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let wfst_set = wfst
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();

    if legacy_set == wfst_set {
        ShadowMismatch::SameSetDifferentOrder
    } else if legacy_top.is_some() && wfst_set.contains(&legacy_top.unwrap_or_default()) {
        ShadowMismatch::LegacyTopFoundInWfst
    } else if wfst_top.is_some() && legacy_set.contains(&wfst_top.unwrap_or_default()) {
        ShadowMismatch::WfstTopFoundInLegacy
    } else if wfst.candidates.is_empty() {
        ShadowMismatch::WfstEmpty
    } else {
        ShadowMismatch::OutputMismatch
    }
}

fn format_failure(failure: &DecodeFailure) -> String {
    match failure {
        DecodeFailure::EmptyResult => "empty_result".to_owned(),
        DecodeFailure::Error(message) => format!("error:{}", sanitize_field(message)),
        DecodeFailure::LowConfidence => "low_confidence".to_owned(),
        DecodeFailure::Timeout => "timeout".to_owned(),
        DecodeFailure::Unavailable => "unavailable".to_owned(),
    }
}

fn sanitize_optional(value: &Option<String>) -> String {
    value
        .as_ref()
        .map(|text| sanitize_field(text))
        .unwrap_or_else(|| "-".to_owned())
}

fn sanitize_field(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            '\t' | '\n' | '\r' => ' ',
            _ => ch,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::{DecodeCandidate, DecodeResult};

    #[test]
    fn summary_counts_rows() {
        let legacy = DecodeResult::success(
            "legacy",
            vec![DecodeCandidate {
                text: "ជា".to_owned(),
                score_bps: None,
                confidence_bps: None,
                segments: Vec::new(),
            }],
            5,
        );
        let wfst = DecodeResult::failed("wfst", DecodeFailure::Timeout, 11);
        let observation = build_shadow_observation(DecoderMode::Shadow, "jea", &legacy, Some(&wfst));

        let mut summary = ShadowSummary::default();
        summary.record(&observation);

        assert_eq!(summary.total, 1);
        assert_eq!(summary.wfst_failed, 1);
    }
}
