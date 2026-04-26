# khPOS Integration Mechanism Note

This note explains the intended mechanism for using `khPOS` data inside `khmerime`.

Scope of this note:

- explain the current decoder mechanism
- explain what `khPOS` changes
- explain the narrower V1 plan
- keep the discussion reviewable by both humans and future Codex runs

Implementation status in this repository:

- build-time compilation of `khPOS` after-replace statistics is implemented
- a compact binary stats artifact is embedded alongside the lexicon
- dominant POS tags for single Khmer words are loaded into the ranked lexicon metadata
- active corpus-LM reranking is intentionally not enabled yet because the first naive weighting destabilized existing WFST phrase-recovery tests

## Attribution

The `khPOS` corpus bundled in this repository is not original work from this project.

The `khPOS` corpus is attributed to:

- Vichet Chea
- Ye Kyaw Thu

Original repository:

- https://github.com/ye-kyaw-thu/khPOS

The corpus documentation also cites the following paper:

- Ye Kyaw Thu, Vichet Chea, Yoshinori Sagisaka, "Comparison of Six POS Tagging Methods on 12K Sentences Khmer Language POS Tagged Corpus", ONA 2017

This note only describes how `khmerime` may consume `khPOS` statistics for ranking. It does not claim authorship of the corpus itself.

Current assumption for V1:

- use only `data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all`
- use only `data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all.tag`
- do not use `before-replace` in V1

## 1. Current Project Mechanism

The current system is mainly a lexicon-guided IME decoder, not a trained end-to-end statistical model.

High-level flow:

```text
roman input
   |
   v
normalize input
   |
   v
segment into plausible spans
   |
   v
retrieve Khmer candidates from lexicon
   |
   v
score and combine candidates
   |
   v
return ranked suggestions
```

More specifically:

```text
user types: knhhomttovsalarien
              |
              v
        composer analyzes input
              |
              v
   possible spans:
   [knhhom] [ttov] [salarien]
              |
              v
   each span retrieves Khmer candidates
              |
              v
   WFST-style beam combines candidate paths
              |
              v
   final rank is based on several score parts
```

The important design point is that the decoder is already structured like a scoring pipeline:

```text
total score
  = chunk score
  + segmentation score
  + lm score
  + pos score
  + history score
```

But today:

- chunk score is active
- segmentation score is active
- history score is active
- LM score exists, but mostly reflects lexicon-derived counts
- POS score exists structurally, but is effectively off

So the project already has slots for corpus-informed ranking, but those slots are underused.

## 2. What khPOS Is Supposed To Do

`khPOS` is not meant to replace the lexicon and not meant to replace the decoder.

The intended use is:

```text
khPOS corpus
   |
   +--> tell us which Khmer words are common
   |
   +--> tell us which Khmer word sequences are common
   |
   +--> tell us which POS tags are common
   |
   +--> tell us which POS transitions are plausible
```

That means `khPOS` acts as a source of ranking priors.

It does not generate the Khmer candidates directly.

Mechanically:

```text
roman input
   |
   v
lexicon + composer produce candidate Khmer paths
   |
   v
khPOS-derived statistics help rerank those paths
   |
   v
better final suggestion ordering
```

This is a much smaller and safer change than training a new model from scratch.

## 3. Proposed V1 Mechanism

V1 should use `khPOS` only as an offline count source.

Training inputs:

```text
after-replace/train.all
after-replace/train.all.tag
```

Build-time compiler output:

```text
$OUT_DIR/khpos.stats.bin
```

The compiler should extract:

- Khmer word unigram counts
- Khmer word bigram counts
- POS unigram counts
- POS bigram counts
- Khmer word -> dominant POS counts

Training flow:

```text
after-replace/train.all
after-replace/train.all.tag
        |
        v
build-time stats compiler
        |
        +--> word unigram counts
        +--> word bigram counts
        +--> tag unigram counts
        +--> tag bigram counts
        +--> word -> tag counts
        |
        v
khpos.stats.bin
```

Runtime flow:

```text
khpos.stats.bin
        |
        v
Rust loader
        |
        v
ranked lexicon metadata receives corpus-derived tag information
```

## 4. What Changes In The Decoder

The candidate generation mechanism should stay the same in V1.

That means:

- composer still finds spans
- lexicon still produces Khmer candidates
- beam search still combines candidate paths

Only the ranking pressure changes.

Before:

```text
candidate score
  = retrieval fit
  + segmentation heuristic
  + light LM from lexicon counts
  + history
```

After the intended V1 activation:

```text
candidate score
  = retrieval fit
  + segmentation heuristic
  + corpus LM from khPOS counts
  + optional tiny POS prior
  + history
```

This means the decoder would ask not only:

```text
"does this candidate match the roman input?"
```

but also:

```text
"does this Khmer sequence look natural in real corpus data?"
```

## 5. ASCII Example

Suppose the decoder is comparing two possible outputs:

```text
Candidate A: ខ្ញុំ ទៅ សាលារៀន
Candidate B: ខ្ញុំ ទៅ សាលា រៀន
```

Current logic:

```text
A score = chunk match + segmentation heuristic + current LM/history
B score = chunk match + segmentation heuristic + current LM/history
winner = whichever total is larger
```

With `khPOS` corpus LM:

```text
A score = old score
        + word_score(ខ្ញុំ)
        + word_score(ទៅ)
        + word_score(សាលារៀន)
        + pair_score(ខ្ញុំ -> ទៅ)
        + pair_score(ទៅ -> សាលារៀន)

B score = old score
        + word_score(ខ្ញុំ)
        + word_score(ទៅ)
        + word_score(សាលា)
        + word_score(រៀន)
        + pair_score(ខ្ញុំ -> ទៅ)
        + pair_score(ទៅ -> សាលា)
        + pair_score(សាលា -> រៀន)
```

So the decoder becomes corpus-informed:

```text
roman fit
   +
Khmer sequence naturalness
   =
better ranking
```

## 6. Where POS Fits

POS should be treated as a second-layer signal, not the first improvement.

Example tagged sequence:

```text
ខ្ញុំ/PRO ទៅ/VB សាលារៀន/NN
```

Then the scoring logic could use POS transitions like:

```text
PRO -> VB -> NN
```

Mechanism:

```text
previous candidate tag ----> current candidate tag
         |                         |
         +------ lookup transition score ------+
                               |
                               v
                      add bonus or penalty
```

But V1 should keep this weak or off because:

- the current data model only has one `pos_tag` per candidate
- multiword phrases do not map cleanly to one POS tag
- corpus LM is more reliable than early POS scoring

So the correct order is:

```text
Current code: compile stats + load metadata
Next step: enable corpus LM with safe weighting
Later: stronger POS transition scoring
```

## 7. Why Using Only after-replace Is Acceptable For V1

For V1 we only need aligned sentence-level counts.

That requires data shaped like:

```text
sentence: word1 word2 word3
tags:     tag1  tag2  tag3
```

Then we can count:

```text
word1
word1 -> word2
tag1
tag1 -> tag2
word1 -> tag1
```

That is already provided by:

- `after-replace/train.all`
- `after-replace/train.all.tag`

So `after-replace` is sufficient for:

- word unigram counts
- word bigram counts
- tag unigram counts
- tag bigram counts
- word -> tag counts

It is not sufficient to answer every corpus question, but it is sufficient for the narrower V1 ranking mechanism.

## 8. What V1 Explicitly Does Not Do

V1 does not:

- retrain a CRF, HMM, or neural POS tagger
- replace the lexicon
- replace the composer
- replace the legacy serving path
- change `data/roman_lookup.csv` format
- depend on `before-replace`

The intended V1 target is:

```text
khPOS after-replace counts
   ->
compiled binary statistics
   ->
safe corpus-informed ranking
```

## 9. Review Questions

Before implementation, reviewers should confirm:

1. Is V1 limited to count-based priors only?
2. Do we agree to use only `after-replace` in V1?
3. Should corpus LM be enabled before POS scoring?
4. Should all rollout remain in `Wfst` / `Shadow` before touching legacy behavior?

If the answer to all four is yes, the next implementation step is straightforward:

```text
compile khPOS stats into khpos.stats.bin
load stats in Rust
keep metadata integration stable
tune corpus LM until shadow/golden tests stay healthy
then enable ranking use
```
