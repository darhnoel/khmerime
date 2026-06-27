//! Spike: byT5 span-proposal provider (candle). Feature-gated behind `byt5`.
//!
//! Loads the distilled s1 byT5 model + Lexicon vocab from env paths and proposes
//! lexicon-constrained Khmer for roman spans via cached constrained beam search
//! (vendored cache-capable T5 in `byt5_t5.rs`). Off by default so other platforms
//! stay ML-free. To be extracted into its own crate later.
//!
//! Env:
//!   KHMERIME_MODEL_DIR  -> dir with config.json + model.safetensors (the s1 checkpoint)
//!   KHMERIME_VOCAB      -> vocab.txt (one Khmer word per line); else <MODEL_DIR>/vocab.txt
//!   KHMERIME_BYT5_BEAMS -> beam width (default 6)

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;

use super::byt5_t5::{Config, T5ForConditionalGeneration};
use super::span_proposal::{SpanProposal, SpanProposalProvider, SpanProposalRequest};

const EOS: u32 = 1; // ByT5: pad=0, eos=1, unk=2, byte -> id = byte + 3
const MAX_LEN: usize = 48;

// ---- trie over each vocab word's target token sequence (bytes+3, then EOS) ----
#[derive(Default)]
struct Node {
    next: HashMap<u32, Node>,
}

fn build_trie(words: &[String]) -> Node {
    let mut root = Node::default();
    for w in words {
        let mut node = &mut root;
        for b in w.bytes() {
            node = node.next.entry(b as u32 + 3).or_default();
        }
        node = node.next.entry(EOS).or_default();
        let _ = node;
    }
    root
}

struct Model {
    inner: T5ForConditionalGeneration,
    device: Device,
}

struct Beam<'a> {
    tokens: Vec<u32>,
    score: f32,
    node: &'a Node,
}

impl Model {
    fn load(dir: &Path) -> anyhow::Result<Self> {
        let device = Device::Cpu;
        let cfg: Config = serde_json::from_str(&std::fs::read_to_string(dir.join("config.json"))?)?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[dir.join("model.safetensors")], DType::F32, &device)?
        };
        let inner = T5ForConditionalGeneration::load(vb, &cfg)?;
        Ok(Self { inner, device })
    }

    // Cached constrained beam search. Returns top-k Khmer words (real Lexicon words).
    fn constrained(&mut self, roman: &str, trie: &Node, beam_width: usize, top_k: usize) -> anyhow::Result<Vec<String>> {
        self.inner.clear_kv_cache();
        let mut input: Vec<u32> = roman.to_lowercase().bytes().map(|b| b as u32 + 3).collect();
        if input.is_empty() {
            return Ok(Vec::new());
        }
        input.push(EOS);
        let input_t = Tensor::new(input.as_slice(), &self.device)?.unsqueeze(0)?;
        let enc = self.inner.encode(&input_t)?;

        let mut beams = vec![Beam { tokens: vec![0], score: 0.0, node: trie }];
        let mut finished: Vec<(Vec<u32>, f32)> = Vec::new();

        while !beams.is_empty() && beams[0].tokens.len() < MAX_LEN {
            let batch = beams.len();
            let last: Vec<u32> = beams.iter().map(|b| *b.tokens.last().unwrap()).collect();
            let dec_in = Tensor::from_vec(last, (batch, 1), &self.device)?;
            let enc_batch = enc.repeat((batch, 1, 1))?;
            let logits = self.inner.decode(&dec_in, &enc_batch)?;
            let vocab = *logits.dims().last().unwrap();
            let rows: Vec<Vec<f32>> = logits.reshape((batch, vocab))?.to_dtype(DType::F32)?.to_vec2()?;

            let mut exp: Vec<(usize, u32, f32)> = Vec::new(); // (beam, token, score)
            for (bi, (beam, lg)) in beams.iter().zip(rows.iter()).enumerate() {
                let mx = lg.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let norm = mx + lg.iter().map(|l| (l - mx).exp()).sum::<f32>().ln();
                for &tok in beam.node.next.keys() {
                    exp.push((bi, tok, beam.score + lg[tok as usize] - norm));
                }
            }
            exp.sort_unstable_by(|a, b| b.2.total_cmp(&a.2).then_with(|| a.1.cmp(&b.1)));

            let mut next_beams: Vec<Beam> = Vec::with_capacity(beam_width);
            let mut parents: Vec<u32> = Vec::with_capacity(beam_width);
            for (rank, &(bi, tok, score)) in exp.iter().enumerate() {
                let parent = &beams[bi];
                if tok == EOS {
                    if rank < beam_width {
                        finished.push((parent.tokens.clone(), score / parent.tokens.len() as f32));
                        if finished.len() > beam_width {
                            let worst = finished
                                .iter()
                                .enumerate()
                                .min_by(|(_, a), (_, b)| a.1.total_cmp(&b.1))
                                .map(|(i, _)| i)
                                .unwrap();
                            finished.swap_remove(worst);
                        }
                    }
                } else {
                    let mut tokens = parent.tokens.clone();
                    tokens.push(tok);
                    next_beams.push(Beam { tokens, score, node: &parent.node.next[&tok] });
                    parents.push(bi as u32);
                    if next_beams.len() == beam_width {
                        break;
                    }
                }
            }
            if !parents.is_empty() {
                let idx = Tensor::new(parents.as_slice(), &self.device)?;
                self.inner.reorder_kv_cache(&idx)?;
            }
            beams = next_beams;
        }

        finished.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        Ok(finished
            .into_iter()
            .take(top_k)
            .map(|(tokens, _)| {
                let bytes: Vec<u8> = tokens[1..].iter().map(|&id| (id - 3) as u8).collect();
                String::from_utf8_lossy(&bytes).into_owned()
            })
            .collect())
    }
}

pub(crate) struct Byt5SpanProposalProvider {
    model: Mutex<Model>,
    trie: Node,
    beam_width: usize,
    top_k: usize,
}

impl Byt5SpanProposalProvider {
    /// Build from env paths. Returns None (provider disabled) if anything is missing.
    pub(crate) fn load_from_env() -> Option<Arc<dyn SpanProposalProvider>> {
        let dir = std::env::var("KHMERIME_MODEL_DIR").ok()?;
        let dir = Path::new(&dir);
        let vocab_path = std::env::var("KHMERIME_VOCAB")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| dir.join("vocab.txt"));
        let words: Vec<String> = std::fs::read_to_string(&vocab_path)
            .ok()?
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        if words.is_empty() {
            return None;
        }
        let model = Model::load(dir).ok()?;
        // Default greedy (beam 1) — fastest, since the provider currently runs on the live
        // keystroke path. Raise via KHMERIME_BYT5_BEAMS once it moves to a refiner tier.
        let beam_width = std::env::var("KHMERIME_BYT5_BEAMS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        Some(Arc::new(Self {
            model: Mutex::new(model),
            trie: build_trie(&words),
            beam_width,
            top_k: 5,
        }))
    }
}

impl SpanProposalProvider for Byt5SpanProposalProvider {
    fn candidate_ends(&self, _full_input: &str, start: usize, chars: &[char]) -> Vec<usize> {
        // Offer sub-word spans so the lattice can assemble multi-word segmentations
        // (knhmttvsalarien -> knhm|ttv|salarien), not just a whole-input span. The decoder's
        // over_budget bail caps the per-span model calls.
        super::span_proposal::candidate_span_ends(start, chars.len())
    }

    fn propose(&self, request: &SpanProposalRequest<'_>) -> Vec<SpanProposal> {
        let Ok(mut model) = self.model.lock() else {
            return Vec::new();
        };
        let words = model
            .constrained(request.span, &self.trie, self.beam_width, self.top_k)
            .unwrap_or_default();
        words
            .into_iter()
            .enumerate()
            .map(|(rank, output)| SpanProposal {
                output,
                recovered_roman: request.span.to_owned(),
                confidence_bps: 9_000u16.saturating_sub((rank as u16) * 1_000).max(5_000),
                source: "byt5",
            })
            .collect()
    }
}
