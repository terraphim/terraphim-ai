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
/// localStorage key for the Brain Lab's trainable second brain.
const STORAGE_KEY_B: &str = "terraphim-megahal-brain-b";
/// localStorage key remembering the chosen personality across reloads.
const PERSONALITY_KEY: &str = "terraphim-megahal-personality";
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

    /// A second default-trained brain for the Brain Lab: identical to Brain
    /// A at creation, so live training visibly diverges its replies.
    #[wasm_bindgen(js_name = labBrain)]
    pub fn lab_brain() -> MegahalBrain {
        MegahalBrain::new("default")
    }

    /// Whether a saved brain exists in localStorage under `key`.
    pub fn has_local_under(key: &str) -> bool {
        window()
            .local_storage()
            .ok()
            .flatten()
            .map(|storage| storage.get_item(key).ok().flatten().is_some())
            .unwrap_or(false)
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
        self.save_local_under(STORAGE_KEY)
    }

    /// Save the brain to localStorage under an explicit key.
    pub fn save_local_under(&self, key: &str) -> Result<(), JsValue> {
        let storage = window()
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("localStorage unavailable"))?;
        storage.set_item(key, &String::from_utf8_lossy(&self.save()))
    }

    /// Load the brain from localStorage (error when absent).
    pub fn load_local(&mut self) -> Result<(), JsValue> {
        self.load_local_under(STORAGE_KEY)
    }

    /// Load the brain from localStorage under an explicit key.
    pub fn load_local_under(&mut self, key: &str) -> Result<(), JsValue> {
        let storage = window()
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("localStorage unavailable"))?;
        let raw = storage
            .get_item(key)?
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

    // Populate the personality selector from the embedded corpora.
    for name in MegahalBrain::list_personalities() {
        let name = name.as_string().expect("personality names are strings");
        let option = document
            .create_element("option")?
            .dyn_into::<web_sys::HtmlOptionElement>()?;
        option.set_value(&name);
        option.set_text(&name);
        personality_select.append_child(&option)?;
    }

    // Restore any saved brain; otherwise honour a remembered personality
    // (or start on the default one).
    let remembered = window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(PERSONALITY_KEY).ok().flatten());
    let start = remembered
        .as_deref()
        .filter(|name| !name.is_empty() && name != &"default".to_string());
    let brain = std::rc::Rc::new(std::cell::RefCell::new(MegahalBrain::new(
        start.unwrap_or("default"),
    )));
    if let Some(name) = start {
        let length = personality_select.length();
        for index in 0..length {
            let matches = personality_select
                .item(index)
                .and_then(|o| o.dyn_into::<web_sys::HtmlOptionElement>().ok())
                .map(|o| o.value() == name)
                .unwrap_or(false);
            if matches {
                personality_select.set_selected_index(index as i32);
                break;
            }
        }
    }
    if MegahalBrain::has_local() {
        let result = brain.borrow_mut().load_local();
        let restored_message = match result {
            Ok(()) => "restored brain from localStorage".to_string(),
            Err(error) => format!("brain restore failed: {error:?}"),
        };
        status.set_text_content(Some(&restored_message));
    } else {
        status.set_text_content(Some(&format!(
            "fresh {} personality",
            start.unwrap_or("default")
        )));
    }

    // Send: read the input, append it to the log, generate the reply. Shared
    // logic closure so the button click and the input's Enter key both work.
    let send_logic: std::rc::Rc<dyn Fn()> = {
        let brain = brain.clone();
        let chat_log = chat_log.clone();
        let input = input.clone();
        let latency = latency.clone();
        std::rc::Rc::new(move || {
            let text = input.value();
            append_chat(&chat_log, "you:", &text, "chat-line chat-you");
            let reply = brain.borrow_mut().reply(&text);
            let ms = brain.borrow().last_reply_ms();
            latency.set_text_content(Some(&format!("{ms:.1} ms")));
            append_chat(&chat_log, "megahal:", &reply, "chat-line chat-bot");
            input.set_value("");
        })
    };
    let send = {
        let logic = send_logic.clone();
        Closure::<dyn FnMut()>::new(move || logic())
    };
    let send_enter = {
        let logic = send_logic.clone();
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Enter" {
                logic();
            }
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

    // Personality switch: reset with the chosen personality and remember it
    // across reloads.
    let switch = {
        let brain = brain.clone();
        let personality_select = personality_select.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let name = personality_select.value();
            brain.borrow_mut().reset(&name, Some(42));
            let _ = window()
                .local_storage()
                .ok()
                .flatten()
                .map(|storage| storage.set_item(PERSONALITY_KEY, &name));
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

    // Enter in the chat input sends, like the button.
    {
        let target: &web_sys::EventTarget = input.dyn_ref().expect("#chat-input is an EventTarget");
        target
            .add_event_listener_with_callback("keydown", send_enter.as_ref().unchecked_ref())
            .expect("chat Enter wires up");
    }

    // Wire every button by id.
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
    std::mem::forget((send, send_enter, train, switch, save_local, load_local));
    wire_brain_lab()?;
    Ok(())
}

/// Brain Lab: two brains share one seed and one input. Brain A is trained on
/// the default personality; Brain B starts blank so training (or a
/// personality switch) visibly changes its replies while Brain A stays put.
fn wire_brain_lab() -> Result<(), JsValue> {
    let document = document();
    let log_a = document.get_element_by_id("log-a").expect("#log-a");
    let log_b = document.get_element_by_id("log-b").expect("#log-b");
    let input = document
        .get_element_by_id("lab-input")
        .expect("#lab-input")
        .dyn_into::<web_sys::HtmlInputElement>()?;
    let train_area = document
        .get_element_by_id("lab-train-text")
        .expect("#lab-train-text")
        .dyn_into::<web_sys::HtmlTextAreaElement>()?;
    let personality = document
        .get_element_by_id("lab-personality")
        .expect("#lab-personality")
        .dyn_into::<web_sys::HtmlSelectElement>()?;
    let latency_a = document.get_element_by_id("latency-a").expect("#latency-a");
    let latency_b = document.get_element_by_id("latency-b").expect("#latency-b");
    let status = document
        .get_element_by_id("lab-status")
        .expect("#lab-status");

    let brain_a = std::rc::Rc::new(std::cell::RefCell::new(MegahalBrain::new("default")));
    // Brain B starts identical to Brain A (same personality, same seed), so
    // live training visibly diverges its replies from A's.
    let brain_b = std::rc::Rc::new(std::cell::RefCell::new(MegahalBrain::lab_brain()));
    if MegahalBrain::has_local_under(STORAGE_KEY_B) {
        let result = brain_b.borrow_mut().load_local_under(STORAGE_KEY_B);
        status.set_text_content(Some(match result {
            Ok(()) => "Brain B restored from localStorage",
            Err(_) => "Brain B: blank",
        }));
    } else {
        status.set_text_content(Some(
            "Both brains start identical -- train Brain B below and watch the replies diverge",
        ));
    }

    // Ask both brains the same question with the same seed position. The
    // logic lives in an Rc closure so both the button click and the input's
    // Enter key can invoke it.
    let ask_logic: std::rc::Rc<dyn Fn()> = {
        let brain_a = brain_a.clone();
        let brain_b = brain_b.clone();
        let log_a = log_a.clone();
        let log_b = log_b.clone();
        let input = input.clone();
        let latency_a = latency_a.clone();
        let latency_b = latency_b.clone();
        std::rc::Rc::new(move || {
            let text = input.value();
            append_chat(&log_a, "you:", &text, "chat-line chat-you");
            append_chat(&log_b, "you:", &text, "chat-line chat-you");
            let reply_a = brain_a.borrow_mut().reply(&text);
            let ms_a = brain_a.borrow().last_reply_ms();
            latency_a.set_text_content(Some(&format!("{ms_a:.1} ms")));
            append_chat(&log_a, "brain A:", &reply_a, "chat-line chat-bot");
            let reply_b = brain_b.borrow_mut().reply(&text);
            let ms_b = brain_b.borrow().last_reply_ms();
            latency_b.set_text_content(Some(&format!("{ms_b:.1} ms")));
            append_chat(&log_b, "brain B:", &reply_b, "chat-line chat-bot");
            input.set_value("");
        })
    };
    let ask = {
        let logic = ask_logic.clone();
        Closure::<dyn FnMut()>::new(move || logic())
    };
    let ask_enter = {
        let logic = ask_logic.clone();
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Enter" {
                logic();
            }
        })
    };

    // Train only Brain B on the pasted text.
    let train = {
        let brain_b = brain_b.clone();
        let train_area = train_area.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let text = train_area.value();
            let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
            brain_b.borrow_mut().learn(&text);
            status.set_text_content(Some(&format!("Brain B trained on {lines} lines")));
            train_area.set_value("");
        })
    };

    // Reset Brain B on a chosen personality.
    let switch = {
        let brain_b = brain_b.clone();
        let personality = personality.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let name = personality.value();
            brain_b.borrow_mut().reset(&name, Some(42));
            status.set_text_content(Some(&format!("Brain B reset on {name}")));
        })
    };

    // Save/load Brain B.
    let save = {
        let brain_b = brain_b.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let result = brain_b.borrow().save_local_under(STORAGE_KEY_B);
            status.set_text_content(Some(match result {
                Ok(()) => "Brain B saved",
                Err(_) => "Brain B save failed",
            }));
        })
    };
    let load = {
        let brain_b = brain_b.clone();
        let status = status.clone();
        Closure::<dyn FnMut()>::new(move || {
            let result = brain_b.borrow_mut().load_local_under(STORAGE_KEY_B);
            status.set_text_content(Some(match result {
                Ok(()) => "Brain B loaded",
                Err(_) => "no saved Brain B found",
            }));
        })
    };

    if let Some(element) = document.get_element_by_id("lab-send") {
        let target: &web_sys::EventTarget = element.dyn_ref().expect("#lab-send is an EventTarget");
        target
            .add_event_listener_with_callback("click", ask.as_ref().unchecked_ref())
            .expect("lab ask wires up");
    }
    // Enter in the lab input asks both brains, like the button.
    {
        let target: &web_sys::EventTarget = input.dyn_ref().expect("#lab-input is an EventTarget");
        target
            .add_event_listener_with_callback("keydown", ask_enter.as_ref().unchecked_ref())
            .expect("lab Enter wires up");
    }
    for (id, closure) in [
        ("lab-train", &train),
        ("lab-switch", &switch),
        ("lab-save", &save),
        ("lab-load", &load),
    ] {
        if let Some(element) = document.get_element_by_id(id) {
            let target: &web_sys::EventTarget = element
                .dyn_ref()
                .unwrap_or_else(|| panic!("{id} is an EventTarget"));
            target
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .expect("lab control wires up");
        }
    }

    std::mem::forget((ask, ask_enter, train, switch, save, load));
    Ok(())
}
