# Provider architecture — a clean boundary for a model

[`model-provider-challenge.md`](model-provider-challenge.md) is the invitation: the engine
has a seam, bring your own model. This document is the **structure** that keeps the seam
clean — so the public repo carries no model, no inference source, and no model runtime
dependency, while anyone can drop a provider in through one documented binary contract, the
same on every platform.

Four properties we want, and how the structure delivers each:

- **No model in the repo** — no weights, no model architecture, no model config in the public tree.
- **No inference source in the repo** — a provider's model code lives in a crate that is never public.
- **The OSS engine stays alive without a model** — the public workspace builds and runs Standard
  mode with no model-runtime dependency anywhere in its graph.
- **Bring your own model, cross-platform** — a published C ABI plus a weightless example
  lets a third party ship their own provider binary.

## Where we are today

The good part: the public engine already contains **no inference code**. `crates/core/src/`
has zero references to any model runtime, weights format, or model architecture — only the
`SpanProposalProvider` trait and the empty `register_span_proposal_provider()` slot. Nothing
about a model leaks today.

The dependency story is already clean too: the optional `model-provider` cargo feature is
**off by default** (`default = []` in both the core crate and the workspace root). A plain
`cargo build` pulls no model-runtime crate and no inference dependency — verified with
`cargo tree -p khmerime_core`, which shows those crates only when `--features model-provider`
is passed explicitly.

The one cosmetic tidiness question — not a leak — is whether the optional inference *dependency
declarations* should appear in the public `Cargo.toml` at all, or move entirely into the
private provider crate, so the public manifest names no model-runtime crate. That is a
manifest-hygiene call, not a correctness one; the default build is already model-free.

## The target boundary — three layers

| Layer | Contains | Ships a model? | Repo |
|---|---|---|---|
| Public engine | decoder, lexicon, the provider **trait** + `register_*()` slot, the C ABI | no | OSS |
| Provider (ours) | a model + its inference code, a weights loader, an arming entry — implements the trait | yes | private |
| Provider (BYO) | anyone's implementation of the same C ABI | yes | third-party |

The seam is already the right shape — a trait, a registration function, and a C entry point
that reads two env vars for the model paths. The change is to **lift the contract into a
public, documented ABI** owned by neither provider, with product-agnostic symbol names.

## Proposed file / folder structure

Legend: **[pub]** public OSS repo · **[priv]** private (separate repo / gitignored) ·
**[byo]** public template so others can bring their own.

```
khmerime/                              # OSS repo — builds & runs with NO model deps
├─ crates/
│  ├─ core/            [pub]           # engine; the optional model-provider feature is removed
│  │  └─ src/decoder/span_proposal.rs  #   trait + register() only
│  ├─ session/        [pub]
│  └─ provider-abi/   [pub]  (NEW)     # the stable C ABI + header, NO implementation
│     ├─ src/lib.rs                    #   extern "C" contract + safe Rust wrappers
│     └─ include/khmerime_provider.h   #   the published header others target
├─ adapters/          [pub]           # macos-imk · linux-ibus · windows-tsf · ios · android
│  └─ …                                #   build against the trait; no model-provider feature
├─ providers/
│  └─ example-provider/ [byo] (NEW)    # reference BYO impl: a trivial lookup, no weights
│     ├─ src/lib.rs                    #   implements the ABI — the copy-me template
│     └─ README.md                     #   "how to bring your own model"
└─ docs/PROVIDER-ABI.md  [pub] (NEW)   # the contract spec

<private>                              # our AI, never public
└─ providers/<name>/    [priv]         # our provider: model + inference code
   ├─ src/lib.rs                        #   impl SpanProposalProvider
   ├─ model/                            #   weights + vocab/index (gitignored)
   └─ build.mk                          #   cross-compile + package per platform
```

Why a separate `provider-abi` crate: today the C entry point lives in our *private* library,
so the contract is defined by private code. Moving the ABI definition + header into a public
crate makes the boundary a first-class, documented artifact that both our provider and any
BYO provider compile against — neither side owns the contract.

## The bring-your-own-model contract

A provider is any dynamic library (`.dylib` / `.so` / `.dll`) that exports these symbols.
Point the two env vars at your model files; the engine links or `dlopen`s the library and
calls init once at startup.

```c
/* docs/PROVIDER-ABI.md — published header: khmerime_provider.h */

/* Called once at engine startup. Reads:
 *   KHMERIME_MODEL_DIR   — directory with your model files
 *   KHMERIME_MODEL_VOCAB — your vocab / index file
 * Registers the provider. Returns true on success; false = engine stays
 * Standard (a safe no-op). Idempotent. */
bool khmerime_provider_init(void);

/* Optional: report ABI version so the engine can reject a mismatch. */
uint32_t khmerime_provider_abi_version(void);
```

Internally a provider implements the Rust `SpanProposalProvider` trait (`candidate_ends` +
`propose`) and calls `register_span_proposal_provider()`. The C ABI is the thin,
language-neutral wrapper — so providers written against any runtime (a neural model, an
n-gram table, a corpus lookup, in any language) are possible. A C ABI (Application Binary
Interface) is a binary-level contract, so a provider compiled separately — a different
compiler, language, or version — still interoperates without being recompiled against the
engine.

The result is three drop-in stories over one contract:

- **ours** — ship our provider library + weights, private.
- **BYO** — a third party ships their own library + weights, targeting the public header.
- **OSS** — no provider present, engine runs Standard. Ships and works with zero model deps.

## Migration steps

1. **Tidy the manifest.** Remove the optional `model-provider` feature and its inference
   dependency declarations from `crates/core/Cargo.toml`, the workspace root, and
   `adapters/macos-imk`, so the public manifests name no model-runtime crate. (Small, safe,
   immediately verifiable: the OSS workspace still builds and passes tests in Standard — it
   already does, since the feature is off by default.)
2. **Add `crates/provider-abi/`.** Define the C entry-point contract + `khmerime_provider.h`
   here, with product-agnostic symbol names (`khmerime_provider_init`, env vars
   `KHMERIME_MODEL_DIR` / `KHMERIME_MODEL_VOCAB`).
3. **Add `providers/example-provider/`.** A tiny public reference provider (a hardcoded
   lookup, no weights) implementing the ABI — proves the seam end to end and is the BYO
   template.
4. **Write `docs/PROVIDER-ABI.md`.** The contract: symbols, env vars, the
   `candidate_ends` / `propose` semantics, the ABI-version rule, and how to build a `.so` /
   `.dylib` / `.dll`.
5. **Keep our provider private.** It stays in a separate repo, targeting the public
   `provider-abi` crate, with weights in a gitignored `model/`.
6. **Per-platform packaging targets** emit the provider library and stage the model into each
   adapter's gitignored drop-in path (macOS app bundle, Linux data dir, Windows install dir).
   See the desktop packaging notes below.

## Desktop packaging (macOS / Linux / Windows)

The engine, the trait, the model, and the init entry point are identical across platforms.
Only *how the provider binary links* and *where the model file sits* differ.

| Platform | Adapter | Link the provider | Model file lives |
|---|---|---|---|
| macOS | `macos-imk` | provider static lib in the IMK `.app` | loose in the app bundle (like iOS) |
| Linux | `linux-ibus` | link or `dlopen` the provider `.so` | `/usr/lib/khmerime/` or an XDG data dir |
| Windows | `windows-tsf` | link or `LoadLibrary` the provider `.dll` | alongside the TSF `.dll` in the install dir |

Hard constraint: **some model runtimes load weights by real filesystem path** and cannot read
from inside an archive. Where that applies, the weights must be a loose file on disk at install
time. Apple bundles are already loose (iOS / macOS); Linux and Windows install them to a known
path; Android is the one exception that extracts from the APK on first launch.

Suggested sequencing when we implement: **macOS first** (highest reuse from iOS — bundle the
model, link the provider library, arm via the bundle path), **Linux second** (the ibus zero-copy
image path fits mmap'ing the model; settle link-vs-`dlopen` here), **Windows last** (newest
adapter, most installer / TSF-registration groundwork).

## Honest threat model

This structure guarantees our **inference source and model architecture never enter the public
repo**, and the OSS build has no model dependency in its graph. It does **not** encrypt the
shipped weights: a user who installs an AI build can still locate the weights on disk and read
them, and can disassemble the provider binary — identical to the iOS / Android builds today.
If weight secrecy from end users is a requirement, that needs encrypted weights or server-side
inference, a separate and larger effort. This design solves source/repo hygiene and BYO
extensibility, which is what was asked.

## See also

- [`model-provider-challenge.md`](model-provider-challenge.md) — the invitation and the seam's
  design principles (engine stays in charge, nothing blocks a keystroke, absent by default).
- [ADR-0016](adr/0016-runtime-model-provider-behind-a-lexicon-verified-marker.md) — why the
  provider is a runtime seam, not a fork.
- [ADR-0006](adr/0006-uniffi-swift-rust-bridge-for-ios.md) — the Swift/Rust bridge, for how
  adapters cross the language boundary.
