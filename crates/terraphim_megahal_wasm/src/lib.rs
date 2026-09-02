//! wasm-bindgen API and Trunk demo for [`terraphim_megahal`] -- a
//! client-side MegaHAL chat in the spirit of megahal.bisks.net: session-local
//! learning, brain save/load, personality selection, no server.
//!
//! JavaScript API (see [`MegahalBrain`]):
//!
//! - `new MegahalBrain(personality)` / `reset(personality, seed)`
//! - `learn(text)` / `reply(text)` (+ `lastReplyMs` latency getter)
//! - `save() -> Uint8Array` / `load(bytes)` and `saveLocal()` / `loadLocal()`
//!   (localStorage)
//! - `listPersonalities()` for the demo selector
//!
//! Deterministic: the PCG32 seed is part of the saved brain, so a restored
//! conversation continues identically. No OS entropy, no clock, no network.
//! The demo UI is deliberately framework-free (web-sys only, matching the
//! org's other Trunk projects such as terraphim-editor) to keep the bundle
//! small.

use wasm_bindgen::prelude::*;

use rand_core::SeedableRng;
use terraphim_megahal::MegaHal;
use terraphim_sooth::DefaultRng;

/// localStorage key for the demo brain.
const STORAGE_KEY: &str = "terraphim-megahal-brain";
/// Default RNG seed when a brain is created fresh.
const DEFAULT_SEED: u64 = 42;

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn document() -> web_sys::Document {
    window().document().expect("no `document` on window")
}

/// A Megahal brain handle for JavaScript.
#[wasm_bindgen]
pub struct MegahalBrain {
    hal: MegaHal,
    rng: DefaultRng,
    seed: u64,
    last_reply_ms: f64,
}

#[wasm_bindgen]
impl MegahalBrain {
    /// Create a brain trained on the named personality (empty string selects
    /// the `default` personality).
    #[wasm_bindgen(constructor)]
    pub fn new(personality: &str) -> MegahalBrain {
        let hal = if personality.is_empty() || personality == "default" {
            MegaHal::new()
        } else {
            let mut hal = MegaHal::blank();
            hal.load_personality(personality)
                .expect("personality is embedded");
            hal
        };
        MegahalBrain {
            hal,
            rng: DefaultRng::seed_from_u64(DEFAULT_SEED),
            seed: DEFAULT_SEED,
            last_reply_ms: 0.0,
        }
    }

    /// Names of the embedded personalities for the selector.
    pub fn list_personalities() -> Vec<JsValue> {
        MegaHal::personality_names()
            .into_iter()
            .map(JsValue::from)
            .collect()
    }

    /// Learn from one line or a multi-line block of training material.
    pub fn learn(&mut self, text: &str) {
        self.hal.train(text);
    }

    /// Generate a reply; an empty input asks for a greeting. Records the
    /// wall-clock reply latency for the demo's latency badge.
    pub fn reply(&mut self, text: &str) -> String {
        let started = window().performance().map(|p| p.now()).unwrap_or_default();
        let reply = if text.trim().is_empty() {
            self.hal.reply(None, &mut self.rng)
        } else {
            self.hal.reply(Some(text), &mut self.rng)
        };
        let finished = window().performance().map(|p| p.now()).unwrap_or_default();
        self.last_reply_ms = finished - started;
        reply
    }

    /// Wall-clock duration of the most recent [`MegahalBrain::reply`], in
    /// milliseconds.
    #[wasm_bindgen(getter, js_name = lastReplyMs)]
    pub fn last_reply_ms(&self) -> f64 {
        self.last_reply_ms
    }

    /// Wipe the brain, reload the named personality and reset the RNG seed.
    pub fn reset(&mut self, personality: &str, seed: Option<u64>) {
        *self = MegahalBrain::new(personality);
        if let Some(seed) = seed {
            self.seed = seed;
            self.rng = DefaultRng::seed_from_u64(seed);
        }
    }

    /// Replace the RNG seed (forking the conversation determinism).
    #[wasm_bindgen(js_name = setSeed)]
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = DefaultRng::seed_from_u64(seed);
    }

    /// Serialise the brain + RNG seed (`MHRS1W1` document: the MHRS1 brain
    /// plus the conversation's seed, so restores continue identically).
    pub fn save(&self) -> Vec<u8> {
        #[derive(serde::Serialize)]
        struct WasmBrainFile<'a> {
            format: &'a str,
            seed: u64,
            brain: serde_json::Value,
        }
        let brain: serde_json::Value =
            serde_json::from_str(&self.hal.save()).expect("MHRS1 document is valid JSON");
        let file = WasmBrainFile {
            format: "MHRS1W1",
            seed: self.seed,
            brain,
        };
        serde_json::to_vec(&file).expect("brain serialisation cannot fail")
    }

    /// Restore a brain saved with [`MegahalBrain::save`].
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        #[derive(serde::Deserialize)]
        struct WasmBrainFile {
            format: String,
            seed: u64,
            brain: serde_json::Value,
        }
        let file: WasmBrainFile = serde_json::from_slice(bytes)
            .map_err(|error| JsValue::from_str(&format!("bad brain document: {error}")))?;
        if file.format != "MHRS1W1" {
            return Err(JsValue::from_str(&format!(
                "unsupported format {}",
                file.format
            )));
        }
        let brain_json = serde_json::to_string(&file.brain).expect("serialising brain cannot fail");
        self.hal
            .load(&brain_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.seed = file.seed;
        self.rng = DefaultRng::seed_from_u64(self.seed);
        Ok(())
    }

    /// Save the brain to localStorage under the demo key.
    pub fn save_local(&self) -> Result<(), JsValue> {
        let storage = window()
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("localStorage unavailable"))?;
        storage.set_item(STORAGE_KEY, &String::from_utf8_lossy(&self.save()))
    }

    /// Load the brain from localStorage (error when absent).
    pub fn load_local(&mut self) -> Result<(), JsValue> {
        let storage = window()
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("localStorage unavailable"))?;
        let raw = storage
            .get_item(STORAGE_KEY)?
            .ok_or_else(|| JsValue::from_str("no saved brain in localStorage"))?;
        self.load(raw.as_bytes())
    }

    /// Whether a saved brain exists in localStorage.
    pub fn has_local() -> bool {
        window()
            .local_storage()
            .ok()
            .flatten()
            .map(|storage| storage.get_item(STORAGE_KEY).ok().flatten().is_some())
            .unwrap_or(false)
    }
}

/// Append a line to the demo chat log.
fn append_chat(log: &web_sys::Element, speaker: &str, text: &str, class_name: &str) {
    let document = document();
    let line = document.create_element("div").expect("div");
    line.set_class_name(class_name);
    line.set_text_content(Some(&format!("{speaker} {text}")));
    log.append_child(&line).expect("append chat line");
    log.set_scroll_top(log.scroll_height());
}

/// Demo bootstrap: wire the static UI in `index.html` to a
/// [`MegahalBrain`]. Single-threaded wasm makes `Rc<RefCell<_>>` sound here.
#[wasm_bindgen(start)]
pub fn demo_main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let document = document();
    let chat_log = document.get_element_by_id("chat-log").expect("#chat-log");
    let input = document
        .get_element_by_id("chat-input")
        .expect("#chat-input")
        .dyn_into::<web_sys::HtmlInputElement>()?;
    let train_area = document
        .get_element_by_id("train-text")
        .expect("#train-text")
        .dyn_into::<web_sys::HtmlTextAreaElement>()?;
    let personality_select = document
        .get_element_by_id("personality-select")
        .expect("#personality-select")
        .dyn_into::<web_sys::HtmlSelectElement>()?;
    let latency = document.get_element_by_id("latency").expect("#latency");
    let status = document.get_element_by_id("status").expect("#status");

    // Populate the personality selector.
    for name in ["default", "sherlock", "pepys", "startrek", "starwars"] {
        let option = document
            .create_element("option")?
            .dyn_into::<web_sys::HtmlOptionElement>()?;
        option.set_value(name);
        option.set_text(name);
        personality_select.append_child(&option)?;
    }

    // Restore any saved brain, else start on the default personality.
    let brain = std::rc::Rc::new(std::cell::RefCell::new(MegahalBrain::new("default")));
    if MegahalBrain::has_local() {
        let result = brain.borrow_mut().load_local();
        let restored_message = match result {
            Ok(()) => "restored brain from localStorage".to_string(),
            Err(error) => format!("brain restore failed: {error:?}"),
        };
        status.set_text_content(Some(&restored_message));
    } else {
        status.set_text_content(Some("fresh default personality"));
    }

    // Send: read the input, append it to the log, generate the reply.
    let send = {
        let brain = brain.clone();
        let chat_log = chat_log.clone();
        let input = input.clone();
        let latency = latency.clone();
        Closure::<dyn FnMut()>::new(move || {
            let text = input.value();
            append_chat(&chat_log, "you:", &text, "chat-line chat-you");
            let reply = brain.borrow_mut().reply(&text);
            let ms = brain.borrow().last_reply_ms();
            latency.set_text_content(Some(&format!("{ms:.1} ms")));
            append_chat(&chat_log, "megahal:", &reply, "chat-line chat-bot");
            input.set_value("");
        })
    };

    // Train: feed every non-empty line of the textarea into the brain.
    let train = {
        let brain = brain.clone();
        let train_area = train_area.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let text = train_area.value();
            let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
            brain.borrow_mut().learn(&text);
            status.set_text_content(Some(&format!("trained on {lines} lines")));
            train_area.set_value("");
        })
    };

    // Personality switch: reset with the chosen personality.
    let switch = {
        let brain = brain.clone();
        let personality_select = personality_select.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let name = personality_select.value();
            brain.borrow_mut().reset(&name, Some(42));
            status.set_text_content(Some(&format!("switched to {name}")));
        })
    };

    // Save to localStorage.
    let save_local = {
        let brain = brain.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let result = brain.borrow().save_local();
            status.set_text_content(Some(match result {
                Ok(()) => "brain saved to localStorage",
                Err(_) => "localStorage save failed",
            }));
        })
    };

    // Load from localStorage.
    let load_local = {
        let brain = brain.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let result = brain.borrow_mut().load_local();
            status.set_text_content(Some(match result {
                Ok(()) => "brain loaded from localStorage",
                Err(_) => "no saved brain found",
            }));
        })
    };

    // Wire every button by id (button-based send keeps the demo
    // keyboard-agnostic and the wiring framework-free).
    for (id, closure) in [
        ("send-button", &send),
        ("train-button", &train),
        ("personality-apply", &switch),
        ("save-local", &save_local),
        ("load-local", &load_local),
    ] {
        if let Some(element) = document.get_element_by_id(id) {
            let target: &web_sys::EventTarget = element
                .dyn_ref()
                .unwrap_or_else(|| panic!("{id} is an EventTarget"));
            target
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .expect("click listener wires up");
        }
    }
    let _ = &document;

    // Keep the closures alive for the page lifetime.
    std::mem::forget((send, train, switch, save_local, load_local));
    Ok(())
}
