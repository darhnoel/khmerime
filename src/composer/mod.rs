use std::collections::HashMap;

use crate::roman_lookup::{normalize, Entry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComposerChunkKind {
    Explicit,
    Exact,
    Hint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComposerChunk {
    pub normalized: String,
    pub start: usize,
    pub end: usize,
    pub kind: ComposerChunkKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComposerAnalysis {
    pub normalized: String,
    pub chunks: Vec<ComposerChunk>,
    pub wfst_chunk_paths: Vec<Vec<ComposerChunk>>,
    pub pending_tail: String,
    pub fully_segmented: bool,
}

impl ComposerAnalysis {
    pub(crate) fn is_multi_chunk(&self) -> bool {
        let all_chunks_are_strong = self
            .chunks
            .iter()
            .all(|chunk| chunk.end.saturating_sub(chunk.start) >= 4);
        self.fully_segmented
            && self.chunks.len() > 1
            && (self
                .chunks
                .iter()
                .any(|chunk| chunk.kind == ComposerChunkKind::Explicit)
                || self.chunks.len() <= 2
                || all_chunks_are_strong)
    }

    pub(crate) fn wfst_phrase_chunks(&self) -> Vec<ComposerChunk> {
        self.wfst_chunk_paths.first().cloned().unwrap_or_default()
    }

    pub(crate) fn all_wfst_phrase_chunks(&self) -> &[Vec<ComposerChunk>] {
        &self.wfst_chunk_paths
    }

    fn primary_wfst_phrase_chunks(&self) -> Vec<ComposerChunk> {
        if self.is_multi_chunk() {
            return self.chunks.clone();
        }
        if self.chunks.is_empty() {
            return Vec::new();
        }

        let total_len = self.normalized.chars().count();
        let weak_index = self
            .chunks
            .iter()
            .position(|chunk| chunk.end.saturating_sub(chunk.start) < 3);
        if let Some(weak_index) = weak_index {
            if weak_index == 0 && self.chunks.len() > 1 {
                let merged_end = self.chunks[1].end;
                let merged_text = self.normalized.chars().take(merged_end).collect::<String>();
                if merged_text.chars().count() >= 3 {
                    let mut hinted = vec![ComposerChunk {
                        normalized: merged_text,
                        start: 0,
                        end: merged_end,
                        kind: ComposerChunkKind::Hint,
                    }];
                    hinted.extend(
                        self.chunks
                            .iter()
                            .skip(2)
                            .filter(|chunk| chunk.end.saturating_sub(chunk.start) >= 3)
                            .cloned(),
                    );
                    return hinted;
                }
            }

            if weak_index > 0 {
                let strong_prefix_count = weak_index;
                let hint_start = if strong_prefix_count >= 2 {
                    self.chunks[weak_index - 1].start
                } else {
                    self.chunks[weak_index - 1].end
                };
                let hint_text = self.normalized.chars().skip(hint_start).collect::<String>();
                if hint_text.chars().count() >= 3 {
                    let keep_until = if strong_prefix_count >= 2 {
                        weak_index - 1
                    } else {
                        weak_index
                    };
                    let mut hinted = self.chunks[..keep_until].to_vec();
                    hinted.push(ComposerChunk {
                        normalized: hint_text,
                        start: hint_start,
                        end: total_len,
                        kind: ComposerChunkKind::Hint,
                    });
                    return hinted;
                }
            }
        }

        let hint_start = self
            .chunks
            .iter()
            .take_while(|chunk| chunk.end.saturating_sub(chunk.start) >= 3)
            .last()
            .map(|chunk| chunk.end)
            .unwrap_or(0);
        if hint_start == 0 || hint_start >= total_len {
            return Vec::new();
        }
        let hint_text = self.normalized.chars().skip(hint_start).collect::<String>();
        if hint_text.chars().count() < 3 {
            return Vec::new();
        }

        let mut hinted = self.chunks.clone();
        hinted.retain(|chunk| chunk.end <= hint_start);
        hinted.push(ComposerChunk {
            normalized: hint_text,
            start: hint_start,
            end: total_len,
            kind: ComposerChunkKind::Hint,
        });
        hinted
    }
}

#[derive(Default)]
struct ComposerNode {
    children: HashMap<char, usize>,
    terminal: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PathState {
    chunk_count: usize,
    squared_len_sum: usize,
    previous: usize,
}

impl PathState {
    fn better_than(self, other: Self) -> bool {
        self.chunk_count < other.chunk_count
            || (self.chunk_count == other.chunk_count && self.squared_len_sum > other.squared_len_sum)
    }
}

pub(crate) struct ComposerTable {
    nodes: Vec<ComposerNode>,
}

impl ComposerTable {
    pub(crate) fn from_entries(entries: &[Entry]) -> Self {
        let mut table = Self {
            nodes: vec![ComposerNode::default()],
        };

        for entry in entries {
            let normalized = normalize(&entry.roman);
            if normalized.is_empty() {
                continue;
            }
            table.insert(&normalized);
        }

        table
    }

    pub(crate) fn analyze(&self, input: &str) -> ComposerAnalysis {
        let normalized = normalize(input);
        if normalized.is_empty() {
            return ComposerAnalysis {
                normalized,
                chunks: Vec::new(),
                wfst_chunk_paths: Vec::new(),
                pending_tail: String::new(),
                fully_segmented: false,
            };
        }

        let explicit_chunks = self.explicit_chunks(&normalized);
        if explicit_chunks.len() > 1 {
            let wfst_chunk_paths = vec![explicit_chunks.clone()];
            return ComposerAnalysis {
                normalized,
                chunks: explicit_chunks,
                wfst_chunk_paths,
                pending_tail: String::new(),
                fully_segmented: true,
            };
        }

        if let Some(chunks) = self.full_segmentation(&normalized) {
            let primary = chunks.clone();
            return ComposerAnalysis {
                normalized,
                chunks,
                wfst_chunk_paths: vec![primary],
                pending_tail: String::new(),
                fully_segmented: true,
            };
        }

        let prefix_chunks = self.longest_prefix_segmentation(&normalized);
        let pending_tail = if let Some(last) = prefix_chunks.last() {
            normalized.chars().skip(last.end).collect::<String>()
        } else {
            normalized.clone()
        };
        let provisional = ComposerAnalysis {
            normalized,
            chunks: prefix_chunks,
            wfst_chunk_paths: Vec::new(),
            pending_tail,
            fully_segmented: false,
        };
        let wfst_chunk_paths = self.wfst_chunk_paths(&provisional);

        ComposerAnalysis {
            wfst_chunk_paths,
            ..provisional
        }
    }

    fn insert(&mut self, input: &str) {
        let mut node_index = 0usize;
        for ch in input.chars() {
            let next = if let Some(&child) = self.nodes[node_index].children.get(&ch) {
                child
            } else {
                let child = self.nodes.len();
                self.nodes.push(ComposerNode::default());
                self.nodes[node_index].children.insert(ch, child);
                child
            };
            node_index = next;
        }
        self.nodes[node_index].terminal = true;
    }

    fn explicit_chunks(&self, normalized: &str) -> Vec<ComposerChunk> {
        let chars = normalized.chars().collect::<Vec<_>>();
        let mut chunks = Vec::new();
        let mut start = 0usize;

        for (index, ch) in chars.iter().enumerate() {
            if *ch == ' ' || *ch == ',' {
                if start < index {
                    chunks.push(ComposerChunk {
                        normalized: chars[start..index].iter().collect(),
                        start,
                        end: index,
                        kind: ComposerChunkKind::Explicit,
                    });
                }
                start = index + 1;
            }
        }

        if start < chars.len() {
            chunks.push(ComposerChunk {
                normalized: chars[start..].iter().collect(),
                start,
                end: chars.len(),
                kind: ComposerChunkKind::Explicit,
            });
        }

        chunks
    }

    fn full_segmentation(&self, normalized: &str) -> Option<Vec<ComposerChunk>> {
        let chars = normalized.chars().collect::<Vec<_>>();
        let states = self.segment_states(&chars);
        let mut state = states[chars.len()]?;
        let mut chunks = Vec::new();
        let mut end = chars.len();

        while end > 0 {
            let start = state.previous;
            chunks.push(ComposerChunk {
                normalized: chars[start..end].iter().collect(),
                start,
                end,
                kind: ComposerChunkKind::Exact,
            });
            end = start;
            state = match states.get(end).and_then(|entry| *entry) {
                Some(previous_state) => previous_state,
                None if end == 0 => break,
                None => return None,
            };
        }

        chunks.reverse();
        Some(chunks)
    }

    fn longest_prefix_segmentation(&self, normalized: &str) -> Vec<ComposerChunk> {
        let chars = normalized.chars().collect::<Vec<_>>();
        let states = self.segment_states(&chars);
        let mut best_end = 0usize;

        for end in (1..=chars.len()).rev() {
            if states[end].is_some() {
                best_end = end;
                break;
            }
        }

        if best_end == 0 {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut end = best_end;
        while end > 0 {
            let state = match states[end] {
                Some(state) => state,
                None => break,
            };
            let start = state.previous;
            chunks.push(ComposerChunk {
                normalized: chars[start..end].iter().collect(),
                start,
                end,
                kind: ComposerChunkKind::Exact,
            });
            end = start;
        }
        chunks.reverse();
        chunks
    }

    fn segment_states(&self, chars: &[char]) -> Vec<Option<PathState>> {
        let mut states = vec![None; chars.len() + 1];
        states[0] = Some(PathState {
            chunk_count: 0,
            squared_len_sum: 0,
            previous: 0,
        });

        for start in 0..chars.len() {
            let Some(prefix_state) = states[start] else {
                continue;
            };

            let mut node_index = 0usize;
            for end in start..chars.len() {
                let Some(&child) = self.nodes[node_index].children.get(&chars[end]) else {
                    break;
                };
                node_index = child;
                if !self.nodes[node_index].terminal {
                    continue;
                }
                let chunk_len = end + 1 - start;
                let next_state = PathState {
                    chunk_count: prefix_state.chunk_count + 1,
                    squared_len_sum: prefix_state.squared_len_sum + chunk_len * chunk_len,
                    previous: start,
                };
                match states[end + 1] {
                    Some(current) if !next_state.better_than(current) => {}
                    _ => states[end + 1] = Some(next_state),
                }
            }
        }

        states
    }

    fn wfst_chunk_paths(&self, analysis: &ComposerAnalysis) -> Vec<Vec<ComposerChunk>> {
        let mut paths = Vec::<Vec<ComposerChunk>>::new();
        let primary = analysis.primary_wfst_phrase_chunks();
        let primary_has_weak = primary
            .iter()
            .any(|chunk| chunk.end.saturating_sub(chunk.start) < 3 || chunk.kind == ComposerChunkKind::Hint);

        for alternate in self.suffix_anchored_paths(analysis) {
            if !alternate.is_empty() && !paths.contains(&alternate) {
                paths.push(alternate);
            }
        }

        if !primary.is_empty() && (!primary_has_weak || !paths.contains(&primary)) {
            paths.push(primary);
        }

        paths
    }

    fn suffix_anchored_paths(&self, analysis: &ComposerAnalysis) -> Vec<Vec<ComposerChunk>> {
        let chars = analysis.normalized.chars().collect::<Vec<_>>();
        if chars.len() < 6 {
            return Vec::new();
        }

        let Some(weak_index) = analysis
            .chunks
            .iter()
            .position(|chunk| chunk.end.saturating_sub(chunk.start) < 3)
        else {
            return Vec::new();
        };
        if weak_index == 0 {
            return Vec::new();
        }

        let prefix_end = analysis.chunks[weak_index - 1].end;
        let prefix = analysis.chunks[..weak_index]
            .iter()
            .filter(|chunk| chunk.end.saturating_sub(chunk.start) >= 3)
            .cloned()
            .collect::<Vec<_>>();
        if prefix.is_empty() || prefix_end >= chars.len() {
            return Vec::new();
        }

        let mut paths = Vec::<Vec<ComposerChunk>>::new();
        for suffix_start in ((prefix_end + 3)..=chars.len().saturating_sub(3)).rev() {
            let suffix_chars = &chars[suffix_start..];
            let Some(mut suffix_chunks) = self.full_segmentation_from_chars(suffix_chars, suffix_start) else {
                continue;
            };
            let middle_chars = &chars[prefix_end..suffix_start];
            let middle = self.middle_wfst_chunks(middle_chars, prefix_end);
            if middle.is_empty() {
                continue;
            }

            let mut path = prefix.clone();
            path.extend(middle);
            path.append(&mut suffix_chunks);
            paths.push(path);
            if paths.len() >= 3 {
                break;
            }
        }

        paths
    }

    fn middle_wfst_chunks(&self, chars: &[char], global_start: usize) -> Vec<ComposerChunk> {
        if chars.is_empty() {
            return Vec::new();
        }
        if let Some(mut exact) = self.full_segmentation_from_chars(chars, global_start) {
            exact.retain(|chunk| chunk.end.saturating_sub(chunk.start) >= 3);
            return exact;
        }

        let prefix = self.longest_prefix_segmentation_from_chars(chars, global_start);
        let covered = prefix.last().map(|chunk| chunk.end).unwrap_or(global_start);
        let mut chunks = prefix;
        if covered < global_start + chars.len() {
            let normalized = chars[covered - global_start..].iter().collect::<String>();
            if normalized.chars().count() >= 3 {
                chunks.push(ComposerChunk {
                    normalized,
                    start: covered,
                    end: global_start + chars.len(),
                    kind: ComposerChunkKind::Hint,
                });
            }
        }
        chunks
    }

    fn full_segmentation_from_chars(&self, chars: &[char], offset: usize) -> Option<Vec<ComposerChunk>> {
        let states = self.segment_states(chars);
        let mut state = states[chars.len()]?;
        let mut chunks = Vec::new();
        let mut end = chars.len();

        while end > 0 {
            let start = state.previous;
            chunks.push(ComposerChunk {
                normalized: chars[start..end].iter().collect(),
                start: offset + start,
                end: offset + end,
                kind: ComposerChunkKind::Exact,
            });
            end = start;
            state = match states.get(end).and_then(|entry| *entry) {
                Some(previous_state) => previous_state,
                None if end == 0 => break,
                None => return None,
            };
        }

        chunks.reverse();
        Some(chunks)
    }

    fn longest_prefix_segmentation_from_chars(&self, chars: &[char], offset: usize) -> Vec<ComposerChunk> {
        let states = self.segment_states(chars);
        let mut best_end = 0usize;

        for end in (1..=chars.len()).rev() {
            if states[end].is_some() {
                best_end = end;
                break;
            }
        }

        if best_end == 0 {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut end = best_end;
        while end > 0 {
            let state = match states[end] {
                Some(state) => state,
                None => break,
            };
            let start = state.previous;
            chunks.push(ComposerChunk {
                normalized: chars[start..end].iter().collect(),
                start: offset + start,
                end: offset + end,
                kind: ComposerChunkKind::Exact,
            });
            end = start;
        }
        chunks.reverse();
        chunks
    }
}

#[cfg(test)]
mod tests {
    use crate::roman_lookup::Transliterator;

    use super::*;

    #[test]
    fn segments_explicit_chunks() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let table = ComposerTable::from_entries(transliterator.entries());
        let analysis = table.analyze("khnhom ttov");
        assert!(analysis.fully_segmented);
        assert_eq!(
            analysis
                .chunks
                .iter()
                .map(|chunk| chunk.normalized.as_str())
                .collect::<Vec<_>>(),
            vec!["khnhom", "ttov"]
        );
    }

    #[test]
    fn segments_exact_concatenated_chunks() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let table = ComposerTable::from_entries(transliterator.entries());
        let analysis = table.analyze("khnhomttov");
        assert!(analysis.fully_segmented);
        assert_eq!(
            analysis
                .chunks
                .iter()
                .map(|chunk| chunk.normalized.as_str())
                .collect::<Vec<_>>(),
            vec!["khnhom", "ttov"]
        );
    }

    #[test]
    fn exposes_wfst_phrase_hints_for_prefix_plus_tail() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let table = ComposerTable::from_entries(transliterator.entries());
        let analysis = table.analyze("khnhomtov");
        let hints = analysis.wfst_phrase_chunks();
        assert_eq!(
            hints.iter().map(|chunk| chunk.normalized.as_str()).collect::<Vec<_>>(),
            vec!["khnhom", "tov"]
        );
        assert_eq!(hints.last().map(|chunk| chunk.kind), Some(ComposerChunkKind::Exact));
    }
}
