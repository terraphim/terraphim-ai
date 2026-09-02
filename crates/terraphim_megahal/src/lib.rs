//! `terraphim_megahal` -- a faithful Rust port of [MegaHAL], Jason Hutchens'
//! second-order Markov chatterbot, built on [`terraphim_sooth`].
//!
//! The engine keeps five predictors (seed, fore, back, case, punc), a string
//! dictionary and the brain's context mapping, and implements `learn` and
//! `reply` as a near-line-by-line port of the upstream Ruby
//! (`lib/megahal/megahal.rb`, Unlicense). The correctness anchor is the Ruby
//! conformance harness: with the same seed, the same training text and the
//! canonical RNG contract (see below), the Rust port produces byte-identical
//! replies (`tests/conformance.rs`).
//!
//! # Determinism contract
//!
//! - All state maps are [`BTreeMap`]s: iteration and serialisation are
//!   always sorted and reproducible.
//! - No OS entropy, no clock: the caller injects any
//!   [`rand_core::RngCore`] (see [`terraphim_sooth::DefaultRng`] for a
//!   seedable PCG32), so native and wasm32 builds behave identically.
//! - The RNG contract mirrors the upstream Ruby under a canonicalisation:
//!   `rand(n)` consumes exactly one `next_u32()` and yields `value % n`
//!   (`n == 0` yields 0), and `Array#shuffle.first` is a descending
//!   Fisher-Yates shuffle. Both are mirrored in the pure-Ruby fixture driver
//!   (`scripts/generate_megahal_fixtures.rb`), which pins them with an
//!   `rng_self_check` vector in the golden fixtures.
//!
//! # Divergences from upstream
//!
//! - Ruby `Marshal`/zip brain files are not readable; the port defines its
//!   own JSON format (version tag `MHRS1`).
//! - CLD language detection is replaced by a Unicode-range heuristic
//!   ([`segment::character_segmentation`]); English behaviour is unchanged.
//!
//! [MegaHAL]: https://github.com/kranzky/megahal

use std::collections::BTreeMap;

use rand_core::Rng;
use serde::{Deserialize, Serialize};
use terraphim_sooth::{Context, Predictor};

mod keyword;
mod keyword_data;
mod personalities;
mod segment;

#[cfg(feature = "automata")]
pub mod automata_bridge;
#[cfg(feature = "persistence")]
pub mod persist;

pub use keyword::extract;
pub use personalities::available as available_personalities;
pub use segment::Decomposed;
pub use segment::decompose;

/// Special dictionary symbol used by failed selections (upstream's
/// `<error>`, which doubles as the predictors' error event).
pub const ERROR_SYMBOL: u32 = 0;
/// Special dictionary symbol delimiting utterances (upstream's `<fence>`).
pub const FENCE: u32 = 1;
/// Special dictionary symbol marking the "other side" of a seed bigram
/// (upstream's `<blank>`).
pub const BLANK: u32 = 2;

/// Number of keyword-seeded candidate utterances generated per reply
/// (upstream generates nine plus one keyword-free candidate).
const CANDIDATE_UTTERANCES: usize = 9;
/// Maximum rolls of the dice per random-walk step before giving up on
/// eliciting a keyword (upstream constant).
const WALK_ATTEMPTS: u32 = 10;
/// Maximum rewrite retries before an utterance is abandoned (upstream
/// allows retries beyond 9 before giving up).
const REWRITE_RETRIES: u32 = 9;

/// A MegaHAL brain: five predictors plus the dictionary and the
/// context-to-id mapping (kept for state fidelity with upstream, which needs
/// it because the C sooth kernel takes scalar context ids).
///
/// The brain mapping is bijective, so this port uses `(u32, u32)` contexts
/// with [`Predictor`] directly; the map is still maintained (and lazily
/// extended, exactly like upstream) so serialised state round-trips and
/// debugging dumps align with the Ruby implementation.
#[derive(Debug, Clone)]
pub struct MegaHal {
    seed: Predictor,
    fore: Predictor,
    back: Predictor,
    case: Predictor,
    punc: Predictor,
    dictionary: BTreeMap<String, u32>,
    decode: BTreeMap<u32, String>,
    brain: BTreeMap<Context, u32>,
    learning: bool,
}

/// Errors surfaced by [`MegaHal::load`] and [`MegaHal::become`].
#[derive(Debug, thiserror::Error)]
pub enum MegahalError {
    /// The brain JSON is not a valid `MHRS1` document.
    #[error("unsupported brain version or malformed brain: {0}")]
    BadBrain(String),
    /// A personality name was not found (upstream raises ArgumentError).
    #[error("no such personality: {0}")]
    NoSuchPersonality(String),
}

/// Portable brain state: everything needed to reconstruct a
/// [`MegaHal`] (five predictor states, dictionary, brain mapping, learning
/// flag), version-tagged on serialisation.
///
/// This is the unit of persistence: [`MegaHal::save`] embeds it in the
/// `MHRS1` JSON document, and the `persistence` feature stores it through
/// `terraphim_persistence` backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MegaHalState {
    pub learning: bool,
    pub dictionary: Vec<(String, u32)>,
    pub brain: Vec<(Context, u32)>,
    pub seed: Predictor,
    pub fore: Predictor,
    pub back: Predictor,
    pub case: Predictor,
    pub punc: Predictor,
}

/// Brain format version tag.
pub const BRAIN_VERSION: &str = "MHRS1";

/// The versioned on-disk document ([`MegaHal::save`] output).
#[derive(Serialize, Deserialize)]
struct BrainFile<'a> {
    version: &'a str,
    #[serde(flatten)]
    state: MegaHalState,
}

impl Default for MegaHal {
    fn default() -> Self {
        Self::new()
    }
}

/// Canonical `rand(n)`: one draw always consumed; `n == 0` yields 0.
/// Mirrored by the Ruby fixture driver (see the crate documentation).
fn rand_below(rng: &mut impl Rng, bound: u32) -> u32 {
    let value = rng.next_u32();
    if bound == 0 { 0 } else { value % bound }
}

/// Canonical `Array#shuffle.first`: descending Fisher-Yates using
/// [`rand_below`], then take element 0. Returns `None` for an empty slice.
fn shuffle_first<T: Copy>(items: &[T], rng: &mut impl Rng) -> Option<T> {
    let mut shuffled = items.to_vec();
    for i in (1..shuffled.len()).rev() {
        let j = rand_below(rng, (i + 1) as u32) as usize;
        shuffled.swap(i, j);
    }
    shuffled.first().copied()
}

impl MegaHal {
    /// A new brain trained on the embedded `:default` personality, mirroring
    /// upstream's `MegaHAL.new`.
    pub fn new() -> Self {
        let mut hal = Self::blank();
        hal.train(personalities::DEFAULT);
        hal
    }

    /// A wiped brain (`<error>`/`<fence>`/`<blank>` dictionary only),
    /// mirroring upstream's `clear`.
    pub fn blank() -> Self {
        let mut hal = Self {
            seed: Predictor::new(),
            fore: Predictor::new(),
            back: Predictor::new(),
            case: Predictor::new(),
            punc: Predictor::new(),
            dictionary: BTreeMap::new(),
            decode: BTreeMap::new(),
            brain: BTreeMap::new(),
            learning: true,
        };
        hal.intern("<error>");
        hal.intern("<fence>");
        hal.intern("<blank>");
        hal
    }

    /// Wipe the brain (dictionary specials only), mirroring upstream `clear`.
    pub fn clear(&mut self) {
        *self = Self::blank();
    }

    /// Whether replies also learn from the input (upstream `learning`).
    pub fn learning(&self) -> bool {
        self.learning
    }

    /// Toggle reply-time learning (upstream `learning=`).
    pub fn set_learning(&mut self, learning: bool) {
        self.learning = learning;
    }

    /// Names of the embedded personalities.
    pub fn personality_names() -> Vec<&'static str> {
        personalities::available()
            .into_iter()
            .map(|(n, _)| n)
            .collect()
    }

    /// Wipe the brain and train on the named personality, mirroring upstream
    /// `become` (renamed: `become` is a reserved keyword in Rust). With the default feature set only `default` is available.
    pub fn load_personality(&mut self, name: &str) -> Result<(), MegahalError> {
        let corpus = personalities::corpus(name)
            .ok_or_else(|| MegahalError::NoSuchPersonality(name.to_string()))?;
        self.clear();
        self.train(corpus);
        Ok(())
    }

    /// Train on a multi-line text: each line is stripped and learned
    /// (mirroring upstream `_train`).
    pub fn train(&mut self, text: &str) {
        for line in text.lines() {
            self.learn(line);
        }
    }

    /// Learn from one line of text, mirroring `reply`'s learning step:
    /// strip, decompose, observe into all five models.
    pub fn learn(&mut self, line: &str) {
        let decomposed = segment::decompose(Some(line.trim()));
        if let (Some(puncs), Some(norms), Some(words)) =
            (decomposed.puncs, decomposed.norms, decomposed.words)
        {
            self.learn_decomposed(&puncs, &norms, &words);
        }
    }

    /// Generate a reply to `input` (`None` asks for a greeting), using the
    /// default error reply `"..."` (upstream default).
    pub fn reply(&mut self, input: Option<&str>, rng: &mut impl Rng) -> String {
        self.reply_with_error(input, rng, "...")
    }

    /// Generate a reply with an explicit error reply for the case where no
    /// candidate utterance can be rewritten.
    pub fn reply_with_error(
        &mut self,
        input: Option<&str>,
        rng: &mut impl Rng,
        error_reply: &str,
    ) -> String {
        self.reply_with_extra_keywords(input, rng, error_reply, &[])
    }

    /// Generate a reply as [`MegaHal::reply_with_error`], but inject
    /// `extra_keywords` (normalised, upper-case words) into the keyword set
    /// used for reply seeding, *in addition* to the words extracted from the
    /// input itself.
    ///
    /// This is the injection point the `automata` feature uses to bias
    /// replies towards knowledge-graph concepts. With an empty slice the
    /// behaviour is identical to [`MegaHal::reply_with_error`], which is why
    /// the Ruby conformance suite is unaffected by the hook.
    pub fn reply_with_extra_keywords(
        &mut self,
        input: Option<&str>,
        rng: &mut impl Rng,
        error_reply: &str,
        extra_keywords: &[String],
    ) -> String {
        let stripped = input.map(str::trim);
        let decomposed = segment::decompose(stripped);

        let mut keyword_words = keyword::extract(decomposed.norms.as_deref());
        for extra in extra_keywords {
            if !keyword_words.contains(extra) {
                keyword_words.push(extra.clone());
            }
        }
        let keyword_symbols: Vec<u32> = keyword_words
            .iter()
            .filter_map(|word| self.dictionary.get(word).copied())
            .collect();
        let input_symbols: Vec<Option<u32>> = decomposed
            .norms
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|norm| self.dictionary.get(norm).copied())
            .collect();

        // Create candidate utterances: nine keyword-seeded plus one
        // keyword-free.
        let mut utterances: Vec<Vec<u32>> = Vec::new();
        for _ in 0..CANDIDATE_UTTERANCES {
            if let Some(utterance) = self.generate(&keyword_symbols, rng) {
                utterances.push(utterance);
            }
        }
        if let Some(utterance) = self.generate(&[], rng) {
            utterances.push(utterance);
        }

        // Drop candidates that simply echo the input symbols.
        let input_flat: Vec<u32> = input_symbols
            .iter()
            .map(|s| s.unwrap_or(ERROR_SYMBOL))
            .collect();
        let input_all_seen = input_symbols.iter().all(|s| s.is_some());
        utterances.retain(|utterance| !(input_all_seen && *utterance == input_flat));

        // Select the best candidate, rewriting until one succeeds.
        let mut reply = None;
        while reply.is_none() && !utterances.is_empty() {
            let index = match self.select_utterance_index(&utterances, &keyword_symbols) {
                Some(index) => index,
                None => break,
            };
            let utterance = utterances[index].clone();
            reply = self.rewrite(&utterance, rng);
            // Ruby's Array#delete removes *all* equal elements.
            utterances.retain(|candidate| *candidate != utterance);
        }

        // Learn from what the user said *after* generating the reply.
        if self.learning
            && decomposed.norms.is_some()
            && let (Some(puncs), Some(norms), Some(words)) =
                (decomposed.puncs, decomposed.norms, decomposed.words)
        {
            self.learn_decomposed(&puncs, &norms, &words);
        }

        reply.unwrap_or_else(|| error_reply.to_string())
    }

    /// Snapshot the brain state (the unit of persistence).
    pub fn state(&self) -> MegaHalState {
        MegaHalState {
            learning: self.learning,
            dictionary: self
                .dictionary
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            brain: self.brain.iter().map(|(k, v)| (*k, *v)).collect(),
            seed: self.seed.clone(),
            fore: self.fore.clone(),
            back: self.back.clone(),
            case: self.case.clone(),
            punc: self.punc.clone(),
        }
    }

    /// Serialise the brain to the `MHRS1` JSON format.
    pub fn save(&self) -> String {
        let file = BrainFile {
            version: BRAIN_VERSION,
            state: self.state(),
        };
        serde_json::to_string(&file).expect("brain serialisation cannot fail")
    }

    /// Restore a brain previously saved with [`MegaHal::save`].
    pub fn load(&mut self, json: &str) -> Result<(), MegahalError> {
        let file: BrainFile = serde_json::from_str(json)
            .map_err(|error| MegahalError::BadBrain(error.to_string()))?;
        if file.version != BRAIN_VERSION {
            return Err(MegahalError::BadBrain(format!(
                "expected version {BRAIN_VERSION}, found {}",
                file.version
            )));
        }
        self.apply_state(file.state);
        Ok(())
    }

    /// Replace the brain state wholesale.
    pub fn apply_state(&mut self, state: MegaHalState) {
        self.learning = state.learning;
        self.dictionary = state.dictionary.into_iter().collect();
        self.decode = self
            .dictionary
            .iter()
            .map(|(word, symbol)| (*symbol, word.clone()))
            .collect();
        self.brain = state.brain.into_iter().collect();
        self.seed = state.seed;
        self.fore = state.fore;
        self.back = state.back;
        self.case = state.case;
        self.punc = state.punc;
    }

    // -- internals ---------------------------------------------------------

    /// Intern a string into the dictionary, assigning the next id (upstream
    /// `@dictionary[word] ||= @dictionary.length`).
    fn intern(&mut self, word: &str) -> u32 {
        if let Some(&symbol) = self.dictionary.get(word) {
            return symbol;
        }
        let symbol = self.dictionary.len() as u32;
        self.dictionary.insert(word.to_string(), symbol);
        self.decode.insert(symbol, word.to_string());
        symbol
    }

    /// Look up (or lazily assign) a brain id for a context (upstream
    /// `@brain[context] ||= @brain.length` -- note upstream does this during
    /// generation as well, so this mutates state during `reply`).
    fn brain_id(&mut self, context: Context) -> u32 {
        let next = self.brain.len() as u32;
        *self.brain.entry(context).or_insert(next)
    }

    fn model(&self, forward: bool) -> &Predictor {
        if forward { &self.fore } else { &self.back }
    }

    /// Decode a symbol back to its string (builds on the reverse dictionary).
    fn decode(&self, symbol: u32) -> &str {
        self.decode.get(&symbol).map(String::as_str).unwrap_or("")
    }

    /// Train all five models from one decomposed sentence (upstream `_learn`).
    fn learn_decomposed(&mut self, puncs: &[String], norms: &[String], words: &[String]) {
        if words.is_empty() {
            return;
        }

        let punc_symbols: Vec<u32> = puncs.iter().map(|p| self.intern(p)).collect();
        let norm_symbols: Vec<u32> = norms.iter().map(|n| self.intern(n)).collect();
        let word_symbols: Vec<u32> = words.iter().map(|w| self.intern(w)).collect();

        // The seed model learns which words appear adjacent to each other:
        // for every norm (and the closing fence) it observes the bigrams
        // around <blank>.
        let mut previous = FENCE;
        for &norm in norm_symbols.iter().chain(std::iter::once(&FENCE)) {
            self.brain_id((previous, BLANK));
            self.seed.observe((previous, BLANK), norm);
            self.brain_id((BLANK, norm));
            self.seed.observe((BLANK, norm), previous);
            previous = norm;
        }

        // Second-order forward model with fence delimiters.
        let mut context = (FENCE, FENCE);
        for &norm in &norm_symbols {
            self.brain_id(context);
            self.fore.observe(context, norm);
            context = (context.1, norm);
        }
        self.brain_id(context);
        self.fore.observe(context, FENCE);

        // Second-order backward model (fills the start of the utterance).
        let mut context = (FENCE, FENCE);
        for &norm in norm_symbols.iter().rev() {
            self.brain_id(context);
            self.back.observe(context, norm);
            context = (context.1, norm);
        }
        self.brain_id(context);
        self.back.observe(context, FENCE);

        // Case model: (previous word, current norm) -> original word.
        let mut previous_word = FENCE;
        for (&word, &norm) in word_symbols.iter().zip(norm_symbols.iter()) {
            self.brain_id((previous_word, norm));
            self.case.observe((previous_word, norm), word);
            previous_word = word;
        }

        // Punctuation model: (word, next word) -> separator.
        let mut context = (FENCE, FENCE);
        for (&punc, &word) in punc_symbols
            .iter()
            .zip(word_symbols.iter().chain(std::iter::once(&FENCE)))
        {
            context = (context.1, word);
            self.brain_id(context);
            self.punc.observe(context, punc);
        }
    }

    /// Choose a keyword at random from the non-auxiliary keywords (upstream
    /// `_select_keyword`).
    fn select_keyword(&self, keyword_symbols: &[u32], rng: &mut impl Rng) -> Option<u32> {
        let eligible: Vec<u32> = keyword_symbols
            .iter()
            .copied()
            .filter(|symbol| {
                !self
                    .decode
                    .get(symbol)
                    .map(|word| keyword::is_auxiliary(word))
                    .unwrap_or(false)
            })
            .collect();
        shuffle_first(&eligible, rng)
    }

    /// Generate one candidate utterance from keyword symbols (upstream
    /// `_generate`).
    fn generate(&mut self, keyword_symbols: &[u32], rng: &mut impl Rng) -> Option<Vec<u32>> {
        let Some(keyword) = self.select_keyword(keyword_symbols, rng) else {
            // No keywords: a plain forward random walk from the fences.
            let results = self.random_walk(true, (FENCE, FENCE), keyword_symbols, rng);
            return if results.is_empty() {
                None
            } else {
                Some(results)
            };
        };

        // Use the seed model to find a word observed adjacent to the keyword.
        let mut contexts: Vec<Option<Context>> =
            vec![Some((BLANK, keyword)), Some((keyword, BLANK))];
        for slot in contexts.iter_mut() {
            let Some(context) = *slot else { continue };
            let count = self.seed.count(context);
            if count > 0 {
                // Upstream passes the *full* count as the selection limit;
                // the cumulative scan then deterministically returns the
                // highest observed symbol for the context.
                if let Some(selected) = self.seed.select_limit(context, count) {
                    let replaced = if context.0 == BLANK {
                        (selected, context.1)
                    } else {
                        (context.0, selected)
                    };
                    *slot = Some(replaced);
                }
            } else {
                *slot = None;
            }
        }

        let candidates: Vec<Context> = contexts.into_iter().flatten().collect();
        let context = shuffle_first(&candidates, rng)?;

        // Glue the backward and forward walks together: the non-fence
        // symbols of the chosen context sit in the middle of the utterance.
        let glue: Vec<u32> = [context.0, context.1]
            .into_iter()
            .filter(|&s| s != FENCE)
            .collect();
        let mut results = self.random_walk(false, (context.1, context.0), keyword_symbols, rng);
        results.reverse();
        results.extend(glue);
        results.extend(self.random_walk(true, context, keyword_symbols, rng));

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    /// Classic Markovian generation: walk the model from `static_context`
    /// until the fence, preferring rolls that elicit an unused keyword
    /// (upstream `_random_walk`).
    fn random_walk(
        &mut self,
        forward: bool,
        static_context: Context,
        keyword_symbols: &[u32],
        rng: &mut impl Rng,
    ) -> Vec<u32> {
        let mut context = static_context;
        self.brain_id(context);
        if self.model(forward).count(context) == 0 {
            return Vec::new();
        }
        let mut local_keywords = keyword_symbols.to_vec();
        let mut results: Vec<u32> = Vec::new();
        loop {
            let mut symbol = ERROR_SYMBOL;
            for _ in 0..WALK_ATTEMPTS {
                self.brain_id(context);
                let count = self.model(forward).count(context);
                // count == 0 draws a wasted roll and yields limit 1; the
                // positional select then returns the error symbol, matching
                // upstream's rand(0)+1 -> error-event path.
                let limit = rand_below(rng, count) + 1;
                symbol = self
                    .model(forward)
                    .select_limit(context, limit)
                    .unwrap_or(ERROR_SYMBOL);
                if local_keywords.contains(&symbol) {
                    // Ruby's Array#delete removes all equal elements (the
                    // keyword list is already de-duplicated, so this removes
                    // exactly the drawn keyword).
                    local_keywords.retain(|&k| k != symbol);
                    break;
                }
            }
            if symbol == ERROR_SYMBOL {
                return Vec::new();
            }
            if symbol == FENCE {
                break;
            }
            results.push(symbol);
            context = (context.1, symbol);
        }
        results
    }

    /// Index of the best-scoring utterance, first maximum winning (upstream
    /// `_select_utterance` + `_calculate_score`).
    fn select_utterance_index(
        &self,
        utterances: &[Vec<u32>],
        keyword_symbols: &[u32],
    ) -> Option<usize> {
        let mut best_score = -1.0_f64;
        let mut best_index = None;
        for (index, utterance) in utterances.iter().enumerate() {
            let score = self.calculate_score(utterance, keyword_symbols);
            if score > best_score {
                best_score = score;
                best_index = Some(index);
            }
        }
        best_index
    }

    /// Accumulated keyword surprise, forward and backward, normalised for
    /// length (upstream `_calculate_score`).
    fn calculate_score(&self, utterance: &[u32], keyword_symbols: &[u32]) -> f64 {
        let mut score = 0.0;

        let mut context = (FENCE, FENCE);
        for &norm in utterance {
            if keyword_symbols.contains(&norm)
                && let Some(surprise) = self.fore.surprise(context, norm)
            {
                score += surprise;
            }
            context = (context.1, norm);
        }

        let mut context = (FENCE, FENCE);
        for &norm in utterance.iter().rev() {
            if keyword_symbols.contains(&norm)
                && let Some(surprise) = self.back.surprise(context, norm)
            {
                score += surprise;
            }
            context = (context.1, norm);
        }

        if utterance.len() >= 8 {
            score /= ((utterance.len() - 1) as f64).sqrt();
        }
        if utterance.len() >= 16 {
            score /= utterance.len() as f64;
        }
        score
    }

    /// Rewrite a normalised utterance to display text via the case and punc
    /// models (upstream `_rewrite`); `None` when rewriting failed.
    fn rewrite(&mut self, norm_symbols: &[u32], rng: &mut impl Rng) -> Option<String> {
        let mut word_symbols: Vec<u32> = Vec::with_capacity(norm_symbols.len());
        let mut retries = 0_u32;
        let mut index = 0_usize;

        while word_symbols.len() != norm_symbols.len() {
            if retries > REWRITE_RETRIES {
                return None;
            }
            let previous_word = if index == 0 {
                FENCE
            } else {
                word_symbols[index - 1]
            };
            let context = (previous_word, norm_symbols[index]);
            self.brain_id(context);
            let count = self.case.count(context);
            let mut failed = count == 0;
            if !failed {
                let limit = rand_below(rng, count) + 1;
                word_symbols.push(
                    self.case
                        .select_limit(context, limit)
                        .unwrap_or(ERROR_SYMBOL),
                );
            }
            if word_symbols.len() == norm_symbols.len() {
                let final_context = (*word_symbols.last()?, FENCE);
                self.brain_id(final_context);
                failed = self.punc.count(final_context) == 0;
            }
            if failed {
                retries += 1;
                word_symbols.clear();
                index = 0;
                continue;
            }
            index += 1;
        }

        // Generate the separators between words. Upstream zips the (n + 1)
        // separators with the n words; Ruby's zip PADS the shorter side with
        // nil (decoded as empty), so the trailing separator is kept.
        let mut punc_symbols: Vec<u32> = Vec::with_capacity(word_symbols.len() + 1);
        let mut context = (FENCE, FENCE);
        for &word in word_symbols.iter().chain(std::iter::once(&FENCE)) {
            context = (context.1, word);
            self.brain_id(context);
            let count = self.punc.count(context);
            let limit = rand_below(rng, count) + 1;
            punc_symbols.push(
                self.punc
                    .select_limit(context, limit)
                    .unwrap_or(ERROR_SYMBOL),
            );
        }

        let mut out = String::new();
        for (index, punc) in punc_symbols.iter().enumerate() {
            out.push_str(self.decode(*punc));
            if let Some(word) = word_symbols.get(index) {
                out.push_str(self.decode(*word));
            }
        }
        Some(out)
    }
}

/// Runs the core engine under `wasm-bindgen-test` so CI can exercise it on
/// `wasm32-unknown-unknown` (`wasm-pack test --headless`), proving the engine
/// never reaches for OS entropy or the clock on that target.
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use rand_core::SeedableRng;
    type DefaultRngSeed = terraphim_sooth::DefaultRng;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn learn_and_reply_work_on_wasm32() {
        let mut hal = MegaHal::blank();
        hal.train("Hello world. I love Rust.\nRust is fun for chatting.");
        let mut rng = DefaultRngSeed::seed_from_u64(42);
        let reply = hal.reply(Some("Tell me about Rust."), &mut rng);
        assert!(!reply.is_empty());
        let greeting = hal.reply(None, &mut rng);
        assert!(!greeting.is_empty());
    }

    #[wasm_bindgen_test]
    fn replies_are_deterministic_on_wasm32() {
        let build = || {
            let mut hal = MegaHal::blank();
            hal.train("Hello world. I love Rust.\nMarkov chains are fun.");
            hal
        };
        let mut a = build();
        let mut b = build();
        let mut rng_a = DefaultRngSeed::seed_from_u64(7);
        let mut rng_b = DefaultRngSeed::seed_from_u64(7);
        assert_eq!(
            a.reply(Some("I love Rust."), &mut rng_a),
            b.reply(Some("I love Rust."), &mut rng_b)
        );
    }

    #[wasm_bindgen_test]
    fn save_load_round_trip_works_on_wasm32() {
        let mut hal = MegaHal::blank();
        hal.train("Hello world. I love Rust.");
        let saved = hal.save();
        let mut restored = MegaHal::blank();
        restored.load(&saved).expect("valid brain");
        assert_eq!(restored.save(), saved);
    }
}
