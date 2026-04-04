use std::collections::HashMap;

use crate::roman_lookup::{normalize, Entry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComposerChunkKind {
    Explicit,
    Exact,
    #[cfg(any(feature = "wfst-decoder", test))]
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
    pub pending_tail: String,
    pub fully_segmented: bool,
}

impl ComposerAnalysis {
    pub(crate) fn is_multi_chunk(&self) -> bool {
        self.fully_segmented
            && self.chunks.len() > 1
            && (self
                .chunks
                .iter()
                .any(|chunk| chunk.kind == ComposerChunkKind::Explicit)
                || self
                    .chunks
                    .iter()
                    .all(|chunk| chunk.end.saturating_sub(chunk.start) >= 3))
    }

    #[cfg(any(feature = "wfst-decoder", test))]
    pub(crate) fn wfst_phrase_chunks(&self) -> Vec<ComposerChunk> {
        if self.is_multi_chunk() {
            return self.chunks.clone();
        }
        if self.chunks.is_empty() {
            return Vec::new();
        }

        let total_len = self.normalized.chars().count();
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
                pending_tail: String::new(),
                fully_segmented: false,
            };
        }

        let explicit_chunks = self.explicit_chunks(&normalized);
        if explicit_chunks.len() > 1 {
            return ComposerAnalysis {
                normalized,
                chunks: explicit_chunks,
                pending_tail: String::new(),
                fully_segmented: true,
            };
        }

        if let Some(chunks) = self.full_segmentation(&normalized) {
            return ComposerAnalysis {
                normalized,
                chunks,
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

        ComposerAnalysis {
            normalized,
            chunks: prefix_chunks,
            pending_tail,
            fully_segmented: false,
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
