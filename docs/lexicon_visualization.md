# Lexicon Visualization

Use this when you want a quick view of relationships inside `data/roman_lookup.csv`.

Run:

```bash
make visualize-lexicon
```

Histogram size is configurable:

```bash
python3 scripts/data/lexicon/visualize_roman_lookup.py --histogram-items 100
```

Optional interactive explorer:

```bash
python3 -m pip install streamlit
make visualize-lexicon-streamlit
```

Outputs are written to:

```text
dist/roman_lookup-viz/
```

Files:

- `index.html`: main visual dashboard for quick review
- `scripts/data/lexicon/visualize_roman_lookup_streamlit.py`: optional interactive Streamlit explorer over the generated TSV files
- `summary.md`: high-level dataset counts
- `pattern_chunks/`: full-dataset pattern summaries in manageable TSV chunks
- `target_alias_counts.tsv`: Khmer targets and how many roman aliases point to them
- `roman_conflicts.tsv`: roman spellings that point to multiple Khmer targets
- `vowel_pairs.tsv`: roman vowel-family -> Khmer vowel-sign correspondence counts
- `priority_roman_conflicts.tsv`: short ambiguous roman forms worth reviewing first
- `rule_collision_families.tsv`: focused collision families for `j/ch`, `t/tt`, `ue/eu`, and `p/bb`
- `rule_collision_j-ch.tsv`: `j/ch`-focused rule review
- `rule_collision_t-tt.tsv`: `t/tt`-focused rule review
- `rule_collision_ue-eu.tsv`: `ue/eu`-focused rule review
- `rule_collision_p-bb.tsv`: `p/bb`-focused rule review
- `priority_family_conflicts.tsv`: short normalized roman families with multi-target conflicts
- `target_families.tsv`: Khmer targets grouped by normalized roman-family variants
- `review_candidates.tsv`: shortlist of likely manual-review hotspots
- `onset_edges.tsv`: common roman onset -> Khmer initial relationships
- `onset_histogram.tsv`: roman onset histogram data
- `vowel_histogram.tsv`: roman vowel-family histogram data
- `khmer_vowel_histogram.tsv`: Khmer vowel-sign signature histogram data
- `coda_histogram.tsv`: roman coda histogram data
- `khmer_final_histogram.tsv`: Khmer final-character histogram data

The script is intentionally dependency-light and uses only Python standard library so it can run in this repo without a notebook setup.
The optional Streamlit app is the interactive chart layer. The static outputs are TSV-first and no longer rely on SVG.

Start with `index.html` when you want an actual visual overview. Start with `priority_roman_conflicts.tsv` when reviewing spelling tolerance or rule collisions. Use
`rule_collision_families.tsv` when you want a tighter review surface around possible tolerance rules.
Then open the per-rule split file for whichever family you want to judge next.
Use `priority_family_conflicts.tsv` when you want broader family-level patterns.
Use `pattern_chunks/` when you want near-total coverage of the dataset in reviewable chunks instead of a single huge file.
Use `vowel_pairs.tsv` when you want the vowel equivalent of the onset relationship view.
