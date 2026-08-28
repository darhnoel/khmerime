//! Explicit, advisory Khmer spelling review.
//!
//! The checker deliberately treats segmentation as a hint: it scans short,
//! overlapping token windows so a phrase such as `សាលារាន` can still surface
//! `សាលារៀន`, even when the typo itself creates unknown token fragments.
//! Results are possible alternatives, never automatic corrections.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use khmer_segmenter::{KhmerSegmenter, SegmenterConfig};
use roman_lookup::Entry;
use serde::{Deserialize, Serialize};
use symspell_rs::{Suggestion, SymSpell, Verbosity};

const MAX_WINDOW_TOKENS: usize = 4;
const MAX_EDIT_DISTANCE: usize = 1;
const GUARDED_MAX_EDIT_DISTANCE: usize = 2;
const GUARDED_MIN_SOURCE_CHARS: usize = 6;
const GUARDED_MIN_FREQUENCY: usize = 100;
const GUARDED_FREQUENCY_LEAD: usize = 4;
const MAX_SUGGESTIONS_PER_ISSUE: usize = 3;
pub(crate) const MAX_SPELL_ISSUES: usize = 20;
const SAVED_WORD_FREQUENCY: usize = 1_000_000;
const MIN_SOURCE_CHARS: usize = 4;
const MODEL_MAX_EDIT_DISTANCE: usize = 1;
const MODEL_MIN_SOURCE_CHARS: usize = 2;
const MODEL_MIN_FREQUENCY: usize = 10;
const MODEL_UNIQUE_SOURCE_CHARS: usize = 8;
const DETECTOR_THRESHOLD: f32 = 0.35;
const DETECTOR_WARNING_THRESHOLD: f32 = 0.5;
const KHMER_SEGMENTER_DATA: &[u8] = include_bytes!("../../local-data/khmer_dictionary.kdict");

fn khmer_segmenter() -> &'static KhmerSegmenter {
    static SEGMENTER: OnceLock<KhmerSegmenter> = OnceLock::new();
    SEGMENTER.get_or_init(|| {
        KhmerSegmenter::from_bytes(KHMER_SEGMENTER_DATA.to_vec(), SegmenterConfig::default())
            .expect("the local evaluation KDIC must be valid")
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SpellReviewStatus {
    #[default]
    Idle,
    Checking,
    Complete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpellIssue {
    pub start: usize,
    pub end: usize,
    pub source: String,
    pub suggestions: Vec<String>,
    pub kind: SpellIssueKind,
    pub confidence_millis: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SpellIssueKind {
    #[default]
    Error,
    Warning,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct DetectorSpan {
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
}

#[derive(Debug, Deserialize)]
struct DetectorResponse {
    detections: Vec<DetectorSpan>,
}

#[derive(Serialize)]
struct DetectorRequest<'a> {
    text: &'a str,
    threshold: f32,
}

// --- 8901 spell-check API (our 0.9857 segmenter + decomposition + RAC lexicon) ---
// The Python service on 8901 owns the good segmenter and the RAC dictionary check;
// the app fetches it and maps `issues` (char offsets + suggestions) to SpellIssue.
#[derive(Serialize)]
struct ApiRequest<'a> {
    text: &'a str,
}

#[derive(Debug, Deserialize)]
struct ApiIssue {
    start: usize,
    end: usize,
    word: String,
    kind: String,
    suggestion: Option<String>,
    #[serde(default)]
    suggestions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    issues: Vec<ApiIssue>,
}

fn api_kind(kind: &str) -> SpellIssueKind {
    match kind {
        "likely-typo" => SpellIssueKind::Error,
        _ => SpellIssueKind::Warning, // "unknown" -> advisory
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpellSegment {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpellCheckResult {
    pub issues: Vec<SpellIssue>,
    pub segments: Vec<SpellSegment>,
    pub detector_status: ContextDetectorStatus,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ContextDetectorStatus {
    #[default]
    NotChecked,
    Connected,
    Unavailable,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpellReview {
    pub status: SpellReviewStatus,
    pub issues: Vec<SpellIssue>,
    pub segments: Vec<SpellSegment>,
    pub active_index: usize,
    pub open_index: Option<usize>,
    pub choice_index: usize,
    pub detector_status: ContextDetectorStatus,
}

impl SpellReview {
    pub(crate) fn checking() -> Self {
        Self {
            status: SpellReviewStatus::Checking,
            ..Self::default()
        }
    }

    pub(crate) fn complete(result: SpellCheckResult) -> Self {
        Self {
            status: SpellReviewStatus::Complete,
            issues: result.issues,
            segments: result.segments,
            active_index: 0,
            open_index: None,
            choice_index: 0,
            detector_status: result.detector_status,
        }
    }

    pub(crate) fn result_bar_visible(&self) -> bool {
        self.status == SpellReviewStatus::Complete
    }

    pub(crate) fn select(&mut self, index: usize, open: bool) {
        if self.issues.is_empty() {
            return;
        }
        self.active_index = index.min(self.issues.len() - 1);
        self.open_index = open.then_some(self.active_index);
        self.choice_index = 0;
    }

    pub(crate) fn dismiss_interaction(&mut self) {
        self.open_index = None;
        self.choice_index = 0;
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        let len = self.issues.len();
        if len == 0 {
            return;
        }
        self.active_index = if delta < 0 {
            (self.active_index + len - 1) % len
        } else {
            (self.active_index + 1) % len
        };
        self.open_index = Some(self.active_index);
        self.choice_index = 0;
    }

    pub(crate) fn choose_suggestion(&mut self, index: usize) {
        let Some(issue) = self.issues.get(self.active_index) else {
            return;
        };
        self.choice_index = index.min(issue.suggestions.len().saturating_sub(1));
    }

    pub(crate) fn ignore(&mut self, index: usize) {
        if index >= self.issues.len() {
            return;
        }
        self.issues.remove(index);
        self.active_index = self.active_index.min(self.issues.len().saturating_sub(1));
        self.open_index = None;
        self.choice_index = 0;
    }

    /// Remove every current issue whose flagged word equals `word` (ignoring all
    /// instances at once, for the session Ignore List).
    pub(crate) fn ignore_word(&mut self, word: &str) {
        self.issues.retain(|issue| issue.source != word);
        self.active_index = self.active_index.min(self.issues.len().saturating_sub(1));
        self.open_index = None;
        self.choice_index = 0;
    }

    pub(crate) fn accept(&mut self, index: usize, replacement: &str, text: &str) -> Option<(String, usize)> {
        let accepted = self.issues.get(index)?.clone();
        let new_text = replace_char_range(text, accepted.start, accepted.end, replacement);
        let old_len = accepted.end.saturating_sub(accepted.start);
        let new_len = replacement.chars().count();
        let delta = new_len as isize - old_len as isize;

        self.issues.remove(index);
        for issue in &mut self.issues {
            if issue.start >= accepted.end {
                issue.start = shift_index(issue.start, delta);
                issue.end = shift_index(issue.end, delta);
            }
        }
        update_segments_after_replacement(&mut self.segments, accepted.start, accepted.end, delta);
        self.active_index = self.active_index.min(self.issues.len().saturating_sub(1));
        self.open_index = None;
        self.choice_index = 0;
        Some((new_text, accepted.start + new_len))
    }
}

pub(crate) fn caret_after_replacement(
    caret: usize,
    replaced_start: usize,
    replaced_end: usize,
    replacement_len: usize,
) -> usize {
    if caret <= replaced_start {
        caret
    } else if caret >= replaced_end {
        let replaced_len = replaced_end.saturating_sub(replaced_start);
        shift_index(caret, replacement_len as isize - replaced_len as isize)
    } else {
        replaced_start + replacement_len
    }
}

#[derive(Debug)]
struct RankedIssue {
    issue: SpellIssue,
    best_distance: usize,
    best_length: usize,
    length_gap: usize,
    token_count: usize,
    best_frequency: usize,
}

pub(crate) fn check_text(
    text: &str,
    entries: &[Entry],
    user_dictionary: &HashMap<String, Vec<String>>,
) -> SpellCheckResult {
    let frequencies = target_frequencies(entries, user_dictionary);
    if frequencies.is_empty() {
        return SpellCheckResult::default();
    }

    let mut spell = SymSpell::new(2, None, 7, 1);
    for (word, frequency) in &frequencies {
        spell.create_dictionary_entry(word, *frequency);
    }

    let mut ranked = Vec::new();
    let mut segments = Vec::new();
    for (run_start, run) in khmer_runs(text) {
        let Ok(segmentation) = khmer_segmenter().segment_detailed(run) else {
            continue;
        };
        let tokens = segmentation.tokens().map(str::to_owned).collect::<Vec<_>>();
        for mapped in segmentation.mapped_segments() {
            let start = run_start + run[..mapped.source_range.start].chars().count();
            let end = run_start + run[..mapped.source_range.end].chars().count();
            segments.push(SpellSegment { start, end });
        }
        ranked.extend(scan_run(run_start, &tokens, &frequencies, &spell));
    }

    ranked.sort_by(|left, right| {
        left.best_distance
            .cmp(&right.best_distance)
            .then_with(|| left.length_gap.cmp(&right.length_gap))
            .then_with(|| right.best_length.cmp(&left.best_length))
            .then_with(|| left.token_count.cmp(&right.token_count))
            .then_with(|| right.best_frequency.cmp(&left.best_frequency))
            .then_with(|| left.issue.start.cmp(&right.issue.start))
    });

    let mut selected = Vec::<SpellIssue>::new();
    for candidate in ranked {
        if selected.len() == MAX_SPELL_ISSUES {
            break;
        }
        let overlaps = selected
            .iter()
            .any(|issue| candidate.issue.start < issue.end && issue.start < candidate.issue.end);
        if !overlaps {
            selected.push(candidate.issue);
        }
    }
    selected.sort_by_key(|issue| issue.start);
    SpellCheckResult {
        issues: selected,
        segments,
        detector_status: ContextDetectorStatus::NotChecked,
    }
}

fn update_segments_after_replacement(segments: &mut Vec<SpellSegment>, start: usize, end: usize, delta: isize) {
    let first = segments
        .iter()
        .position(|segment| start < segment.end && segment.start < end);
    let last = segments
        .iter()
        .rposition(|segment| start < segment.end && segment.start < end);

    if let (Some(first), Some(last)) = (first, last) {
        let merged_start = segments[first].start;
        let merged_end = shift_index(segments[last].end, delta);
        segments.splice(
            first..=last,
            [SpellSegment {
                start: merged_start,
                end: merged_end,
            }],
        );
        for segment in segments.iter_mut().skip(first + 1) {
            segment.start = shift_index(segment.start, delta);
            segment.end = shift_index(segment.end, delta);
        }
    }
}

fn target_frequencies(entries: &[Entry], user_dictionary: &HashMap<String, Vec<String>>) -> HashMap<String, usize> {
    let mut frequencies = HashMap::<String, usize>::new();
    for entry in entries {
        if is_khmer_word(&entry.target) {
            frequencies
                .entry(entry.target.clone())
                .and_modify(|current| *current = (*current).max(entry.frequency as usize))
                .or_insert(entry.frequency.max(1) as usize);
        }
    }
    for word in user_dictionary.values().flatten() {
        if is_khmer_word(word) {
            frequencies.insert(word.clone(), SAVED_WORD_FREQUENCY);
        }
    }
    frequencies
}

/// True if `word` can be split into a sequence of two or more known dictionary
/// words (a valid compound). Word-break dynamic programming over character
/// prefixes; each piece must be a key in `frequencies`. A single-word cover is
/// handled by the caller's `contains_key`, so here we only accept covers of two
/// or more pieces, and require each piece to be at least 2 characters so a
/// misspelling cannot be "validated" by chaining tiny fragments.
fn decomposes_into_known(word: &str, frequencies: &HashMap<String, usize>) -> bool {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    if n < 4 {
        return false;
    }
    // pieces[i] = smallest number of known words covering chars[..i], or usize::MAX
    let mut pieces = vec![usize::MAX; n + 1];
    pieces[0] = 0;
    const MIN_PIECE: usize = 2;
    const MAX_PIECE: usize = 16;
    for i in MIN_PIECE..=n {
        for j in (0..=i.saturating_sub(MIN_PIECE)).rev() {
            if i - j > MAX_PIECE {
                break;
            }
            if pieces[j] == usize::MAX {
                continue;
            }
            let piece: String = chars[j..i].iter().collect();
            if frequencies.contains_key(&piece) {
                pieces[i] = pieces[i].min(pieces[j] + 1);
            }
        }
    }
    // covered by 2+ known words
    pieces[n] != usize::MAX && pieces[n] >= 2
}

fn scan_run(
    run_start: usize,
    tokens: &[String],
    frequencies: &HashMap<String, usize>,
    spell: &SymSpell,
) -> Vec<RankedIssue> {
    let mut token_starts = Vec::with_capacity(tokens.len());
    let mut offset = run_start;
    for token in tokens {
        token_starts.push(offset);
        offset += token.chars().count();
    }

    let mut results = Vec::new();
    for start in 0..tokens.len() {
        let max_end = (start + MAX_WINDOW_TOKENS).min(tokens.len());
        for end in start + 1..=max_end {
            let source = tokens[start..end].concat();
            let source_length = source.chars().count();
            // Skip a window that is a single known word, OR that DECOMPOSES into a
            // sequence of known words (a valid compound). Without the decomposition
            // check, every compound not present as one dictionary headword
            // (សាច់ញត្តិ = សាច់ + ញត្តិ, ជាការ = ជា + ការ) is falsely flagged.
            if source_length < MIN_SOURCE_CHARS
                || frequencies.contains_key(&source)
                || decomposes_into_known(&source, frequencies)
            {
                continue;
            }
            let longest_component = tokens[start..end]
                .iter()
                .map(|token| token.chars().count())
                .max()
                .unwrap_or(0);
            let lookup_distance = if end - start > 1 && source_length >= GUARDED_MIN_SOURCE_CHARS {
                GUARDED_MAX_EDIT_DISTANCE
            } else {
                MAX_EDIT_DISTANCE
            };
            let mut suggestions = spell.lookup(&source, Verbosity::All, lookup_distance, &None, Some(12), false);
            suggestions.retain(|item| {
                item.distance > 0
                    && item.count > 1
                    && (end - start == 1 || item.term.chars().count() > longest_component)
            });
            sort_suggestions(&mut suggestions);
            deduplicate_suggestions(&mut suggestions);
            if let Some(best_distance) = suggestions.first().map(|suggestion| suggestion.distance) {
                suggestions.retain(|suggestion| suggestion.distance == best_distance);
            }
            if suggestions
                .first()
                .is_some_and(|best| best.distance == GUARDED_MAX_EDIT_DISTANCE)
                && !distance_two_suggestion_is_confident(&suggestions)
            {
                continue;
            }
            suggestions.truncate(MAX_SUGGESTIONS_PER_ISSUE);
            let Some(best) = suggestions.first() else {
                continue;
            };
            let best_distance = best.distance;
            let best_length = best.term.chars().count();
            let best_frequency = best.count;

            let start_char = token_starts[start];
            let end_char = token_starts[end - 1] + tokens[end - 1].chars().count();
            results.push(RankedIssue {
                issue: SpellIssue {
                    start: start_char,
                    end: end_char,
                    source,
                    suggestions: suggestions.into_iter().map(|item| item.term).collect(),
                    kind: SpellIssueKind::Error,
                    confidence_millis: None,
                },
                best_distance,
                best_length,
                length_gap: best_length.abs_diff(source_length),
                token_count: end - start,
                best_frequency,
            });
        }
    }
    results
}

fn distance_two_suggestion_is_confident(suggestions: &[Suggestion]) -> bool {
    let Some(best) = suggestions.first() else {
        return false;
    };
    best.distance == GUARDED_MAX_EDIT_DISTANCE
        && best.count >= GUARDED_MIN_FREQUENCY
        && suggestions
            .iter()
            .skip(1)
            .filter(|candidate| candidate.distance == GUARDED_MAX_EDIT_DISTANCE)
            .all(|candidate| best.count >= candidate.count.saturating_mul(GUARDED_FREQUENCY_LEAD))
}

pub(crate) fn combine_detector_result(
    text: &str,
    entries: &[Entry],
    user_dictionary: &HashMap<String, Vec<String>>,
    mut result: SpellCheckResult,
    detections: &[DetectorSpan],
) -> SpellCheckResult {
    result.detector_status = ContextDetectorStatus::Connected;
    if detections.is_empty() || result.segments.is_empty() {
        return result;
    }

    let frequencies = target_frequencies(entries, user_dictionary);
    let mut spell = SymSpell::new(2, None, 7, 1);
    for (word, frequency) in &frequencies {
        spell.create_dictionary_entry(word, *frequency);
    }

    for hotspot in merge_detector_spans(detections) {
        let overlapping_segments = result
            .segments
            .iter()
            .enumerate()
            .filter(|(_, segment)| ranges_overlap(segment.start, segment.end, hotspot.start, hotspot.end))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let (Some(&first_segment), Some(&last_segment)) = (overlapping_segments.first(), overlapping_segments.last())
        else {
            continue;
        };

        let known_span =
            smallest_known_covering_span(text, &result.segments, first_segment, last_segment, &frequencies);
        let guided = (known_span.is_none() || hotspot.confidence < DETECTOR_WARNING_THRESHOLD)
            .then(|| {
                best_model_guided_candidate(
                    text,
                    &result.segments,
                    first_segment,
                    last_segment,
                    hotspot.start,
                    hotspot.end,
                    &spell,
                )
            })
            .flatten();
        let confidence_millis = Some((hotspot.confidence.clamp(0.0, 1.0) * 1_000.0).round() as u16);
        if guided.is_none()
            && hotspot.confidence < DETECTOR_WARNING_THRESHOLD
            && sentence_has_stronger_detection(text, &hotspot, detections)
        {
            continue;
        }
        let issue = if let Some(candidate) = guided {
            SpellIssue {
                start: candidate.start,
                end: candidate.end,
                source: slice_char_range(text, candidate.start, candidate.end),
                suggestions: candidate.suggestions,
                kind: SpellIssueKind::Error,
                confidence_millis,
            }
        } else {
            let (start, end) =
                known_span.unwrap_or((result.segments[first_segment].start, result.segments[last_segment].end));
            SpellIssue {
                start,
                end,
                source: slice_char_range(text, start, end),
                suggestions: Vec::new(),
                kind: SpellIssueKind::Warning,
                confidence_millis,
            }
        };
        result
            .issues
            .retain(|existing| !ranges_overlap(existing.start, existing.end, hotspot.start, hotspot.end));
        if !result
            .issues
            .iter()
            .any(|existing| ranges_overlap(existing.start, existing.end, issue.start, issue.end))
        {
            result.issues.push(issue);
        }
    }
    result.issues.sort_by_key(|issue| issue.start);
    result.issues.truncate(MAX_SPELL_ISSUES);
    result
}

pub(crate) fn mark_detector_unavailable(mut result: SpellCheckResult) -> SpellCheckResult {
    result.detector_status = ContextDetectorStatus::Unavailable;
    result
}

#[derive(Debug)]
struct GuidedCandidate {
    start: usize,
    end: usize,
    suggestions: Vec<String>,
    distance: usize,
    length_gap: usize,
    frequency: usize,
}

fn smallest_known_covering_span(
    text: &str,
    segments: &[SpellSegment],
    first_segment: usize,
    last_segment: usize,
    frequencies: &HashMap<String, usize>,
) -> Option<(usize, usize)> {
    let first_start = first_segment.saturating_sub(MAX_WINDOW_TOKENS - 1);
    let last_end = (last_segment + MAX_WINDOW_TOKENS).min(segments.len());
    let mut matches = Vec::new();
    for start_index in first_start..=first_segment {
        for end_index in last_segment + 1..=last_end {
            if end_index - start_index > MAX_WINDOW_TOKENS
                || segments[start_index..end_index]
                    .windows(2)
                    .any(|pair| pair[0].end != pair[1].start)
            {
                continue;
            }
            let start = segments[start_index].start;
            let end = segments[end_index - 1].end;
            let source = slice_char_range(text, start, end);
            if source.chars().count() >= MODEL_MIN_SOURCE_CHARS && frequencies.contains_key(&source) {
                matches.push((start, end));
            }
        }
    }
    matches.sort_by_key(|(start, end)| end - start);
    matches.into_iter().next()
}

fn best_model_guided_candidate(
    text: &str,
    segments: &[SpellSegment],
    first_segment: usize,
    last_segment: usize,
    hotspot_start: usize,
    hotspot_end: usize,
    spell: &SymSpell,
) -> Option<GuidedCandidate> {
    let first_start = first_segment.saturating_sub(MAX_WINDOW_TOKENS - 1);
    let last_end = (last_segment + MAX_WINDOW_TOKENS).min(segments.len());
    let mut candidates = Vec::new();
    for start_index in first_start..=first_segment {
        for end_index in last_segment + 1..=last_end {
            if end_index - start_index > MAX_WINDOW_TOKENS {
                continue;
            }
            if segments[start_index..end_index]
                .windows(2)
                .any(|pair| pair[0].end != pair[1].start)
            {
                continue;
            }
            let start = segments[start_index].start;
            let end = segments[end_index - 1].end;
            let source = slice_char_range(text, start, end);
            if source.chars().count() < MODEL_MIN_SOURCE_CHARS {
                continue;
            }
            let longest_component = segments[start_index..end_index]
                .iter()
                .map(|segment| segment.end.saturating_sub(segment.start))
                .max()
                .unwrap_or(0);
            if let Some(candidate) = guided_candidate_for_span(
                &source,
                start,
                end,
                hotspot_start,
                hotspot_end,
                end_index - start_index > 1,
                longest_component,
                spell,
            ) {
                candidates.push(candidate);
            }
        }
    }

    // A typo can make Viterbi attach a valid prefix to the previous token, as
    // in `ពីសា|គ|ល|វិទ្យាល័យ`. Search character boundaries inside the hotspot
    // token so the intended `សាគលវិទ្យាល័យ` span remains recoverable.
    let fallback_first = first_segment
        .checked_sub(1)
        .filter(|previous| segments[*previous].end == segments[first_segment].start)
        .unwrap_or(first_segment);
    let char_start = segments[fallback_first].start;
    let char_end = segments[(last_segment + MAX_WINDOW_TOKENS - 1).min(segments.len() - 1)].end;
    for start in char_start..=hotspot_start {
        for end in hotspot_end..=char_end {
            if end.saturating_sub(start) < MODEL_UNIQUE_SOURCE_CHARS || end.saturating_sub(start) > 24 {
                continue;
            }
            let source = slice_char_range(text, start, end);
            if let Some(candidate) =
                guided_candidate_for_span(&source, start, end, hotspot_start, hotspot_end, false, 0, spell)
            {
                candidates.push(candidate);
            }
        }
    }
    candidates.sort_by(|left, right| {
        left.distance
            .cmp(&right.distance)
            .then_with(|| left.length_gap.cmp(&right.length_gap))
            .then_with(|| (right.end - right.start).cmp(&(left.end - left.start)))
            .then_with(|| right.frequency.cmp(&left.frequency))
    });
    candidates.into_iter().next()
}

fn guided_candidate_for_span(
    source: &str,
    start: usize,
    end: usize,
    hotspot_start: usize,
    hotspot_end: usize,
    joined_segments: bool,
    longest_component: usize,
    spell: &SymSpell,
) -> Option<GuidedCandidate> {
    let mut suggestions = spell.lookup(source, Verbosity::All, MODEL_MAX_EDIT_DISTANCE, &None, Some(12), false);
    suggestions.retain(|item| {
        item.distance > 0
            && (!joined_segments || item.term.chars().count() > longest_component)
            && edit_touches_hotspot(
                source,
                &item.term,
                hotspot_start.saturating_sub(start),
                hotspot_end.saturating_sub(start),
            )
    });
    suggestions.sort_by(|left, right| {
        left.distance
            .cmp(&right.distance)
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| {
                left.term
                    .chars()
                    .count()
                    .abs_diff(source.chars().count())
                    .cmp(&right.term.chars().count().abs_diff(source.chars().count()))
            })
            .then_with(|| left.term.cmp(&right.term))
    });
    deduplicate_suggestions(&mut suggestions);
    suggestions.truncate(MAX_SUGGESTIONS_PER_ISSUE);
    let best = suggestions.first()?;
    let runner_up_frequency = suggestions.get(1).map(|item| item.count).unwrap_or(0);
    let dominant = best.count >= MODEL_MIN_FREQUENCY && best.count >= runner_up_frequency.saturating_mul(4);
    let uniquely_located = source.chars().count() >= MODEL_UNIQUE_SOURCE_CHARS && suggestions.len() == 1;
    if !dominant && !uniquely_located {
        return None;
    }
    Some(GuidedCandidate {
        start,
        end,
        suggestions: suggestions.iter().map(|item| item.term.clone()).collect(),
        distance: best.distance,
        length_gap: best.term.chars().count().abs_diff(source.chars().count()),
        frequency: best.count,
    })
}

fn edit_touches_hotspot(source: &str, candidate: &str, hotspot_start: usize, hotspot_end: usize) -> bool {
    let source_chars = source.chars().collect::<Vec<_>>();
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let prefix = source_chars
        .iter()
        .zip(&candidate_chars)
        .take_while(|(left, right)| left == right)
        .count();
    match candidate_chars.len().cmp(&source_chars.len()) {
        std::cmp::Ordering::Equal => hotspot_start <= prefix && prefix < hotspot_end,
        std::cmp::Ordering::Greater => hotspot_start <= prefix && prefix <= hotspot_end,
        std::cmp::Ordering::Less => hotspot_start <= prefix && prefix < hotspot_end,
    }
}

fn merge_detector_spans(detections: &[DetectorSpan]) -> Vec<DetectorSpan> {
    let mut sorted = detections.to_vec();
    sorted.sort_by_key(|span| span.start);
    let mut merged = Vec::<DetectorSpan>::new();
    for span in sorted {
        if let Some(previous) = merged.last_mut() {
            if span.start <= previous.end {
                previous.end = previous.end.max(span.end);
                previous.confidence = previous.confidence.max(span.confidence);
                continue;
            }
        }
        merged.push(span);
    }
    merged
}

fn sentence_has_stronger_detection(text: &str, hotspot: &DetectorSpan, detections: &[DetectorSpan]) -> bool {
    let chars = text.chars().collect::<Vec<_>>();
    let sentence_start = chars[..hotspot.start.min(chars.len())]
        .iter()
        .rposition(|char| is_sentence_ending(*char))
        .map(|index| index + 1)
        .unwrap_or(0);
    let search_start = hotspot.end.min(chars.len());
    let sentence_end = chars[search_start..]
        .iter()
        .position(|char| is_sentence_ending(*char))
        .map(|offset| search_start + offset + 1)
        .unwrap_or(chars.len());
    detections.iter().any(|span| {
        span.confidence >= DETECTOR_WARNING_THRESHOLD && sentence_start <= span.start && span.start < sentence_end
    })
}

fn is_sentence_ending(char: char) -> bool {
    matches!(char, '។' | '៕' | '?' | '!' | '\n')
}

fn ranges_overlap(left_start: usize, left_end: usize, right_start: usize, right_end: usize) -> bool {
    left_start < right_end && right_start < left_end
}

fn slice_char_range(text: &str, start: usize, end: usize) -> String {
    text.chars().skip(start).take(end.saturating_sub(start)).collect()
}

fn resolve_service_endpoint(
    configured: Option<&str>,
    protocol: &str,
    hostname: &str,
    port: u16,
    path: &str,
) -> String {
    if let Some(endpoint) = configured {
        return endpoint.to_owned();
    }
    let host = if hostname.contains(':') {
        format!("[{hostname}]")
    } else {
        hostname.to_owned()
    };
    format!("{protocol}//{host}:{port}{path}")
}

#[cfg(target_arch = "wasm32")]
fn browser_service_endpoint(
    configured: Option<&str>, port: u16, path: &str
) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "service endpoint has no window".to_owned())?;
    let location = window.location();
    let protocol = location
        .protocol()
        .map_err(|error| format!("read page protocol: {error:?}"))?;
    let hostname = location
        .hostname()
        .map_err(|error| format!("read page hostname: {error:?}"))?;
    Ok(resolve_service_endpoint(
        configured, &protocol, &hostname, port, path,
    ))
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn detect_contextual_errors(text: &str) -> Result<Vec<DetectorSpan>, String> {
    use js_sys::wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let endpoint = browser_service_endpoint(
        option_env!("KHMERIME_DETECTOR_URL"), 8898, "/detect",
    )?;
    let body = serde_json::to_string(&DetectorRequest {
        text,
        threshold: DETECTOR_THRESHOLD,
    })
    .map_err(|error| format!("serialize detector request: {error}"))?;
    let options = RequestInit::new();
    options.set_method("POST");
    options.set_mode(RequestMode::Cors);
    options.set_body(&JsValue::from_str(&body));
    let request = Request::new_with_str_and_init(&endpoint, &options)
        .map_err(|error| format!("create detector request: {error:?}"))?;
    request
        .headers()
        .set("content-type", "application/json")
        .map_err(|error| format!("set detector request header: {error:?}"))?;
    let window = web_sys::window().ok_or_else(|| "detector fetch has no window".to_owned())?;
    let response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|error| format!("detector network error: {error:?}"))?;
    let response: Response = response
        .dyn_into()
        .map_err(|_| "detector response cast failed".to_owned())?;
    if !response.ok() {
        return Err(format!("detector returned HTTP {}", response.status()));
    }
    let response_text = JsFuture::from(
        response
            .text()
            .map_err(|error| format!("read detector response: {error:?}"))?,
    )
    .await
    .map_err(|error| format!("await detector response: {error:?}"))?
    .as_string()
    .ok_or_else(|| "detector response was not text".to_owned())?;
    let response: DetectorResponse =
        serde_json::from_str(&response_text).map_err(|error| format!("parse detector response: {error}"))?;
    Ok(response.detections)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn detect_contextual_errors(_text: &str) -> Result<Vec<DetectorSpan>, String> {
    Err("contextual detector HTTP client is available in the web build".to_owned())
}

/// Spell-check the whole text via the 8901 API (our 0.9857 segmenter +
/// decomposition + RAC lexicon). Returns a ready SpellCheckResult with issues
/// (char offsets + suggestions). This REPLACES the local Rust segmenter + the
/// neural detector path for spell review.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn check_via_api(text: &str) -> Result<SpellCheckResult, String> {
    use js_sys::wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let endpoint = browser_service_endpoint(
        option_env!("KHMERIME_SPELLCHECK_URL"), 8901, "/check",
    )?;
    let body = serde_json::to_string(&ApiRequest { text })
        .map_err(|error| format!("serialize spellcheck request: {error}"))?;
    let options = RequestInit::new();
    options.set_method("POST");
    options.set_mode(RequestMode::Cors);
    options.set_body(&JsValue::from_str(&body));
    let request = Request::new_with_str_and_init(&endpoint, &options)
        .map_err(|error| format!("create spellcheck request: {error:?}"))?;
    request
        .headers()
        .set("content-type", "application/json")
        .map_err(|error| format!("set spellcheck request header: {error:?}"))?;
    let window = web_sys::window().ok_or_else(|| "spellcheck fetch has no window".to_owned())?;
    let response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|error| format!("spellcheck network error: {error:?}"))?;
    let response: Response = response
        .dyn_into()
        .map_err(|_| "spellcheck response cast failed".to_owned())?;
    if !response.ok() {
        return Err(format!("spellcheck returned HTTP {}", response.status()));
    }
    let response_text = JsFuture::from(
        response
            .text()
            .map_err(|error| format!("read spellcheck response: {error:?}"))?,
    )
    .await
    .map_err(|error| format!("await spellcheck response: {error:?}"))?
    .as_string()
    .ok_or_else(|| "spellcheck response was not text".to_owned())?;
    let parsed: ApiResponse =
        serde_json::from_str(&response_text).map_err(|error| format!("parse spellcheck response: {error}"))?;

    let issues = parsed
        .issues
        .into_iter()
        .take(MAX_SPELL_ISSUES)
        .map(|issue| {
            // Prefer the ranked `suggestions` list (ambiguous typos have several);
            // fall back to the single `suggestion` for older API responses.
            let suggestions = if issue.suggestions.is_empty() {
                issue.suggestion.into_iter().collect()
            } else {
                issue.suggestions
            };
            SpellIssue {
                start: issue.start,
                end: issue.end,
                source: issue.word,
                suggestions,
                kind: api_kind(&issue.kind),
                confidence_millis: None,
            }
        })
        .collect();
    Ok(SpellCheckResult {
        issues,
        segments: Vec::new(),
        detector_status: ContextDetectorStatus::Connected,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn check_via_api(_text: &str) -> Result<SpellCheckResult, String> {
    Err("spellcheck HTTP client is available in the web build".to_owned())
}

fn sort_suggestions(suggestions: &mut [Suggestion]) {
    suggestions.sort_by(|left, right| {
        left.distance
            .cmp(&right.distance)
            .then_with(|| right.term.chars().count().cmp(&left.term.chars().count()))
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.term.cmp(&right.term))
    });
}

fn deduplicate_suggestions(suggestions: &mut Vec<Suggestion>) {
    let mut seen = HashSet::new();
    suggestions.retain(|item| seen.insert(item.term.clone()));
}

fn khmer_runs(text: &str) -> Vec<(usize, &str)> {
    let mut runs = Vec::new();
    let mut start_byte = None;
    let mut start_char = 0;
    let mut char_index = 0;
    for (byte_offset, ch) in text.char_indices() {
        match (start_byte, is_khmer_word_char(ch)) {
            (None, true) => {
                start_byte = Some(byte_offset);
                start_char = char_index;
            }
            (Some(run_byte), false) => {
                runs.push((start_char, &text[run_byte..byte_offset]));
                start_byte = None;
            }
            _ => {}
        }
        char_index += 1;
    }
    if let Some(run_byte) = start_byte {
        runs.push((start_char, &text[run_byte..]));
    }
    runs
}

fn is_khmer_word(input: &str) -> bool {
    !input.is_empty() && input.chars().all(is_khmer_word_char)
}

fn is_khmer_word_char(ch: char) -> bool {
    ('\u{1780}'..='\u{17d3}').contains(&ch)
}

fn replace_char_range(text: &str, start: usize, end: usize, replacement: &str) -> String {
    let prefix = text.chars().take(start).collect::<String>();
    let suffix = text.chars().skip(end).collect::<String>();
    format!("{prefix}{replacement}{suffix}")
}

fn shift_index(index: usize, delta: isize) -> usize {
    if delta < 0 {
        index.saturating_sub(delta.unsigned_abs())
    } else {
        index.saturating_add(delta as usize)
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn yield_before_check() {
    gloo_timers::future::TimeoutFuture::new(20).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn yield_before_check() {
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn wait_for_clear_confirmation() {
    gloo_timers::future::TimeoutFuture::new(1_800).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn wait_for_clear_confirmation() {
    tokio::time::sleep(std::time::Duration::from_millis(1_800)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_service_endpoint_uses_page_hostname() {
        assert_eq!(
            resolve_service_endpoint(None, "http:", "192.168.40.240", 8901, "/check"),
            "http://192.168.40.240:8901/check"
        );
        assert_eq!(
            resolve_service_endpoint(
                Some("http://model.local:8091/check"),
                "http:",
                "192.168.40.240",
                8901,
                "/check"
            ),
            "http://model.local:8091/check"
        );
    }

    fn entry(target: &str, frequency: u32) -> Entry {
        Entry {
            roman: target.to_owned(),
            target: target.to_owned(),
            frequency,
            frequency_lang: "km".to_owned(),
        }
    }

    fn school_entries() -> Vec<Entry> {
        vec![
            entry("សាលា", 1_825),
            entry("រាន", 111),
            entry("សាលារៀន", 14),
            entry("នៅ", 4_000),
            entry("ក្បែរ", 500),
            entry("ផ្ទះ", 2_000),
        ]
    }

    #[test]
    fn overlapping_tokens_surface_longer_one_edit_word() {
        let result = check_text("សាលារាននៅក្បែរផ្ទះ", &school_entries(), &HashMap::new());
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].source, "សាលារាន");
        assert_eq!(
            result.issues[0].suggestions.first().map(String::as_str),
            Some("សាលារៀន")
        );
        assert_eq!(result.segments.len(), 5);
    }

    #[test]
    fn unknown_single_token_surfaces_one_edit_word() {
        let result = check_text("សាលារៀណ", &school_entries(), &HashMap::new());
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].source, "សាលារៀណ");
        assert_eq!(
            result.issues[0].suggestions.first().map(String::as_str),
            Some("សាលារៀន")
        );
    }

    #[test]
    fn correct_long_word_is_not_flagged() {
        let result = check_text("សាលារៀននៅក្បែរផ្ទះ", &school_entries(), &HashMap::new());
        assert!(result.issues.is_empty());
    }

    #[test]
    fn reconstructed_long_typo_can_use_a_guarded_two_edit_lookup() {
        let entries = crate::engine(roman_lookup::DecoderMode::Legacy).entries();
        let result = check_text("តែនៅខ្វះពត៍មាន", entries, &HashMap::new());
        let issue = result
            .issues
            .iter()
            .find(|issue| issue.source == "ពត៍មាន")
            .expect("the reconstructed typo should be checked as one source");
        assert_eq!(issue.suggestions.first().map(String::as_str), Some("ព័ត៌មាន"));

        let clean = check_text("តែនៅខ្វះព័ត៌មាន", entries, &HashMap::new());
        assert!(clean.issues.iter().all(|issue| issue.source != "ព័ត៌មាន"));
    }

    #[test]
    fn saved_word_suppresses_possible_correction() {
        let saved = HashMap::from([("salaran".to_owned(), vec!["សាលារាន".to_owned()])]);
        let result = check_text("សាលារាន", &school_entries(), &saved);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn accepting_issue_replaces_span_and_shifts_following_issue() {
        let mut review = SpellReview::complete(SpellCheckResult {
            issues: vec![
                SpellIssue {
                    start: 0,
                    end: 2,
                    source: "កខ".to_owned(),
                    suggestions: vec!["កខគ".to_owned()],
                    kind: SpellIssueKind::Error,
                    confidence_millis: None,
                },
                SpellIssue {
                    start: 3,
                    end: 5,
                    source: "ឃង".to_owned(),
                    suggestions: vec!["ឃងច".to_owned()],
                    kind: SpellIssueKind::Error,
                    confidence_millis: None,
                },
            ],
            segments: vec![SpellSegment { start: 0, end: 2 }, SpellSegment { start: 3, end: 5 }],
            detector_status: ContextDetectorStatus::NotChecked,
        });
        let (text, caret) = review.accept(0, "កខគ", "កខ ឃង").unwrap();
        assert_eq!(text, "កខគ ឃង");
        assert_eq!(caret, 3);
        assert_eq!((review.issues[0].start, review.issues[0].end), (4, 6));
        assert_eq!((review.segments[0].start, review.segments[0].end), (0, 3));
        assert_eq!((review.segments[1].start, review.segments[1].end), (4, 6));
    }

    #[test]
    fn replacement_preserves_and_adjusts_the_users_caret() {
        assert_eq!(caret_after_replacement(2, 4, 7, 5), 2);
        assert_eq!(caret_after_replacement(5, 4, 7, 5), 9);
        assert_eq!(caret_after_replacement(12, 4, 7, 5), 14);
        assert_eq!(caret_after_replacement(12, 4, 9, 2), 9);
    }

    #[test]
    fn model_hotspot_can_join_short_segments_into_confirmed_error() {
        let base = SpellCheckResult {
            issues: Vec::new(),
            segments: vec![SpellSegment { start: 0, end: 2 }, SpellSegment { start: 2, end: 3 }],
            detector_status: ContextDetectorStatus::NotChecked,
        };
        let result = combine_detector_result(
            "របស",
            &[entry("របស់", 70_801)],
            &HashMap::new(),
            base,
            &[DetectorSpan {
                start: 2,
                end: 3,
                confidence: 0.97,
            }],
        );
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].source, "របស");
        assert_eq!(result.issues[0].suggestions, vec!["របស់"]);
        assert_eq!(result.issues[0].kind, SpellIssueKind::Error);
    }

    #[test]
    fn unresolved_model_hotspot_becomes_warning_for_whole_segment() {
        let base = SpellCheckResult {
            issues: Vec::new(),
            segments: vec![SpellSegment { start: 0, end: 3 }],
            detector_status: ContextDetectorStatus::NotChecked,
        };
        let result = combine_detector_result(
            "មនោ",
            &[entry("សាលារៀន", 14)],
            &HashMap::new(),
            base,
            &[DetectorSpan {
                start: 2,
                end: 3,
                confidence: 0.82,
            }],
        );
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].source, "មនោ");
        assert!(result.issues[0].suggestions.is_empty());
        assert_eq!(result.issues[0].kind, SpellIssueKind::Warning);
    }

    #[test]
    fn real_lexicon_confirms_missing_sign_in_rbos() {
        let entries = crate::engine(roman_lookup::DecoderMode::Legacy).entries();
        let base = check_text("របស", entries, &HashMap::new());
        let result = combine_detector_result(
            "របស",
            entries,
            &HashMap::new(),
            base,
            &[DetectorSpan {
                start: 2,
                end: 3,
                confidence: 0.97,
            }],
        );
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].source, "របស");
        assert_eq!(result.issues[0].suggestions.first().map(String::as_str), Some("របស់"));
        assert_eq!(result.issues[0].kind, SpellIssueKind::Error);
    }

    #[test]
    fn low_confidence_short_typo_requires_a_confirmed_repair() {
        let entries = crate::engine(roman_lookup::DecoderMode::Legacy).entries();
        let base = check_text("លោស្រី", entries, &HashMap::new());
        let result = combine_detector_result(
            "លោស្រី",
            entries,
            &HashMap::new(),
            base,
            &[DetectorSpan {
                start: 1,
                end: 2,
                confidence: 0.390,
            }],
        );
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].source, "លោ");
        assert!(result.issues[0].suggestions.is_empty());
        assert_eq!(result.issues[0].kind, SpellIssueKind::Warning);
    }

    #[test]
    fn low_confidence_long_typo_uses_edit_location_to_confirm_repair() {
        let entries = crate::engine(roman_lookup::DecoderMode::Legacy).entries();
        let text = "សាគលវិទ្យាល័យ";
        let base = check_text(text, entries, &HashMap::new());
        let result = combine_detector_result(
            text,
            entries,
            &HashMap::new(),
            base,
            &[DetectorSpan {
                start: 2,
                end: 3,
                confidence: 0.467,
            }],
        );
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].source, text);
        assert_eq!(
            result.issues[0].suggestions.first().map(String::as_str),
            Some("សាកលវិទ្យាល័យ")
        );
        assert_eq!(result.issues[0].kind, SpellIssueKind::Error);
    }

    #[test]
    fn model_hotspot_recovers_word_boundary_inside_merged_prefix_segment() {
        let entries = crate::engine(roman_lookup::DecoderMode::Legacy).entries();
        let text = "ពីសាគលវិទ្យាល័យ";
        let base = check_text(text, entries, &HashMap::new());
        let result = combine_detector_result(
            text,
            entries,
            &HashMap::new(),
            base,
            &[DetectorSpan {
                start: 4,
                end: 5,
                confidence: 0.467,
            }],
        );
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].source, "សាគលវិទ្យាល័យ");
        assert_eq!(
            result.issues[0].suggestions.first().map(String::as_str),
            Some("សាកលវិទ្យាល័យ")
        );
        assert_eq!(result.issues[0].kind, SpellIssueKind::Error);
    }

    #[test]
    fn low_confidence_unconfirmed_hotspot_is_hidden_beside_stronger_sentence_signal() {
        let text = "ឯកឧត្តម សាលារាន";
        let low = DetectorSpan {
            start: 5,
            end: 6,
            confidence: 0.376,
        };
        let high_start = text.chars().position(|char| char == 'រ').unwrap();
        let high = DetectorSpan {
            start: high_start,
            end: high_start + 1,
            confidence: 0.99,
        };
        assert!(sentence_has_stronger_detection(text, &low, &[low.clone(), high]));
    }
}
