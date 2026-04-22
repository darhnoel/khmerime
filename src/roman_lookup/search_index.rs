use super::*;

#[derive(Clone, Debug)]
pub(super) struct SearchIndex {
    gsize_l: usize,
    gsize_u: usize,
    use_levenshtein: bool,
    exact: HashMap<String, String>,
    grams: HashMap<String, Vec<(usize, usize)>>,
    items: BTreeMap<usize, Vec<(f64, String)>>,
}

impl SearchIndex {
    pub(super) fn new(items: &[String], use_levenshtein: bool, gsize_l: usize, gsize_u: usize) -> Self {
        let mut index = Self {
            gsize_l,
            gsize_u,
            use_levenshtein,
            exact: HashMap::new(),
            grams: HashMap::new(),
            items: BTreeMap::new(),
        };

        for item in items {
            index.add(item);
        }

        index
    }

    fn add(&mut self, item: &str) {
        let normalized = normalize(item);
        if self.exact.contains_key(&normalized) {
            return;
        }
        for size in self.gsize_l..=self.gsize_u {
            self.add_with_size(item, size);
        }
        self.exact.insert(normalized, item.to_owned());
    }

    fn add_with_size(&mut self, item: &str, size: usize) {
        let normalized = normalize(item);
        let grams = ngram_counts(&normalized, size);
        let rows = self.items.entry(size).or_insert_with(Vec::new);
        let row_index = rows.len();
        rows.push((0.0, String::new()));

        let mut magnitude = 0f64;
        for (gram, count) in grams {
            magnitude += (count * count) as f64;
            self.grams.entry(gram).or_insert_with(Vec::new).push((row_index, count));
        }

        rows[row_index] = (magnitude.sqrt(), normalized);
    }

    pub(super) fn get(&self, query: &str, threshold: f64) -> Option<Vec<(f64, String)>> {
        for size in (self.gsize_l..=self.gsize_u).rev() {
            let matches = self.get_with_size(query, size, threshold);
            if let Some(ref found) = matches {
                if !found.is_empty() {
                    return matches;
                }
            }
        }
        None
    }

    fn get_with_size(&self, query: &str, size: usize, threshold: f64) -> Option<Vec<(f64, String)>> {
        let normalized = normalize(query);
        let grams = ngram_counts(&normalized, size);
        let rows = self.items.get(&size)?;

        let mut scores = HashMap::<usize, usize>::new();
        let mut seen_rows = Vec::<usize>::new();
        let mut magnitude = 0f64;

        for (gram, count) in &grams {
            magnitude += (*count * *count) as f64;
            if let Some(items) = self.grams.get(gram) {
                for &(row, row_count) in items {
                    let entry = scores.entry(row).or_insert_with(|| {
                        seen_rows.push(row);
                        0
                    });
                    *entry += count * row_count;
                }
            }
        }

        if scores.is_empty() {
            return None;
        }

        let query_norm = magnitude.sqrt();
        let mut ranked = seen_rows
            .into_iter()
            .map(|row| {
                let dot = scores[&row];
                let (item_norm, value) = &rows[row];
                (dot as f64 / (query_norm * *item_norm), value.clone())
            })
            .collect::<Vec<_>>();

        ranked.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());

        if self.use_levenshtein {
            ranked = ranked
                .into_iter()
                .take(50)
                .map(|(_, candidate)| (similarity(&candidate, &normalized), candidate))
                .collect::<Vec<_>>();
            ranked.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
        }

        Some(
            ranked
                .into_iter()
                .filter_map(|(score, candidate)| {
                    if score < threshold {
                        return None;
                    }
                    self.exact.get(&candidate).cloned().map(|original| (score, original))
                })
                .collect(),
        )
    }
}

fn ngram_counts(input: &str, size: usize) -> Vec<(String, usize)> {
    let mut padded = format!("-{}-", normalize(input));
    if padded.len() < size {
        padded.extend(std::iter::repeat('-').take(size - padded.len()));
    }

    let chars = padded.chars().collect::<Vec<_>>();
    let mut counts = Vec::<(String, usize)>::new();
    let mut positions = HashMap::<String, usize>::new();
    for start in 0..=chars.len().saturating_sub(size) {
        let gram = chars[start..start + size].iter().collect::<String>();
        if let Some(&position) = positions.get(&gram) {
            counts[position].1 += 1;
        } else {
            positions.insert(gram.clone(), counts.len());
            counts.push((gram, 1));
        }
    }
    counts
}

pub(crate) fn similarity(left: &str, right: &str) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }

    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    let mut prev = (0..=left.len()).collect::<Vec<_>>();
    let mut curr = vec![0usize; left.len() + 1];

    for (row, right_char) in right.iter().enumerate() {
        curr[0] = row + 1;
        for (col, left_char) in left.iter().enumerate() {
            curr[col + 1] = if left_char == right_char {
                prev[col]
            } else {
                1 + prev[col].min(prev[col + 1]).min(curr[col])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    let distance = prev[left.len()] as f64;
    let denominator = left.len().max(right.len()) as f64;
    1.0 - distance / denominator
}
