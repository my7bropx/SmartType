/// SmartType IBus engine.
///
/// Registers as an IBus engine via D-Bus.  IBus sends key events through
/// ProcessKeyEvent; the engine buffers alphabetic input as preedit text,
/// populates an IBus lookup table with candidates, and commits the chosen
/// (or autocorrected) text on selection or word boundary.
///
/// IBus positions the candidate window at the application's reported caret
/// rectangle — no X11 drawing needed here.
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, RwLock,
};

use anyhow::{Context, Result};
use tokio::sync::Mutex;
use zbus::{interface, ObjectServer, SignalContext};
use zvariant::{Array, Dict, OwnedObjectPath, Signature, StructureBuilder, Value};

use smarttype_core::{AutocorrectEngine, Config, WordCompleter};

// ── X11 keysym constants ──────────────────────────────────────────────────────

const KEY_BACKSPACE: u32 = 0xff08;
const KEY_TAB: u32 = 0xff09;
const KEY_RETURN: u32 = 0xff0d;
const KEY_ESCAPE: u32 = 0xff1b;
const KEY_SPACE: u32 = 0x0020;
const KEY_1: u32 = 0x0031;
const KEY_5: u32 = 0x0035;
const KEY_LEFT: u32 = 0xff51;
const KEY_UP: u32 = 0xff52;
const KEY_RIGHT: u32 = 0xff53;
const KEY_DOWN: u32 = 0xff54;
const KEY_HOME: u32 = 0xff50;
const KEY_END: u32 = 0xff57;
const KEY_DELETE: u32 = 0xffff;
const KEY_PAGE_UP: u32 = 0xff55;
const KEY_PAGE_DOWN: u32 = 0xff56;

const MOD_SHIFT: u32 = 1;
/// IBus sets bit 30 on key-release events.
const MOD_RELEASE: u32 = 1 << 30;

const MIN_PREFIX_LEN: usize = 2;
const MAX_SUGGESTIONS: usize = 5;

// ── IBus GVariant type builders ───────────────────────────────────────────────
//
// IBus serialises its GObject types as nested GVariants with a string type-name
// header.  All signals carry `v` (variant) arguments whose inner type is the
// serialised GObject struct.

fn sig(s: &'static str) -> Signature<'static> {
    Signature::try_from(s).expect("static signature is valid")
}

fn empty_sv_dict() -> Value<'static> {
    Value::Dict(Dict::new(sig("s"), sig("v")))
}

/// Builds an IBusAttrList variant (no attributes).
fn ibus_attr_list() -> Value<'static> {
    Value::Structure(
        StructureBuilder::new()
            .add_field(Value::from("IBusAttrList"))
            .add_field(empty_sv_dict())
            .add_field(Value::Array(Array::new(sig("v"))))
            .build(),
    )
}

/// Builds an IBusText variant wrapping `text`.
fn ibus_text(text: String) -> Value<'static> {
    Value::Structure(
        StructureBuilder::new()
            .add_field(Value::from("IBusText"))
            .add_field(empty_sv_dict())
            .add_field(Value::from(text))
            .add_field(Value::Value(Box::new(ibus_attr_list())))
            .build(),
    )
}

/// Builds an IBusLookupTable variant from `candidates` (up to MAX_SUGGESTIONS).
fn ibus_lookup_table(candidates: &[String]) -> Value<'static> {
    let mut cands = Array::new(sig("v"));
    for word in candidates.iter().take(MAX_SUGGESTIONS) {
        let _ = cands.append(Value::Value(Box::new(ibus_text(word.clone()))));
    }
    Value::Structure(
        StructureBuilder::new()
            .add_field(Value::from("IBusLookupTable"))
            .add_field(empty_sv_dict())
            .add_field(Value::from(MAX_SUGGESTIONS as u32)) // page_size
            .add_field(Value::from(0u32))                   // cursor_pos
            .add_field(Value::from(true))                   // cursor_visible
            .add_field(Value::from(false))                  // round
            .add_field(Value::from(-1i32))                  // orientation: system default
            .add_field(Value::Array(cands))                 // candidates
            .add_field(Value::Array(Array::new(sig("v")))) // labels (empty)
            .build(),
    )
}

// ── Engine state ──────────────────────────────────────────────────────────────

struct EngineState {
    word_buffer: String,
    suggestions: Vec<String>,
}

impl EngineState {
    fn new() -> Self {
        Self { word_buffer: String::new(), suggestions: Vec::new() }
    }

    fn refresh_suggestions(&mut self, completer: &WordCompleter) {
        self.suggestions = if self.word_buffer.len() >= MIN_PREFIX_LEN {
            completer.suggest(&self.word_buffer, MAX_SUGGESTIONS)
        } else {
            Vec::new()
        };
    }

    fn clear(&mut self) {
        self.word_buffer.clear();
        self.suggestions.clear();
    }
}

// ── SmartEngine: org.freedesktop.IBus.Engine ─────────────────────────────────

struct SmartEngine {
    state: Arc<Mutex<EngineState>>,
    completer: Arc<RwLock<WordCompleter>>,
    autocorrect_fn: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
}

impl SmartEngine {
    fn new(
        completer: Arc<RwLock<WordCompleter>>,
        autocorrect_fn: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(EngineState::new())),
            completer,
            autocorrect_fn,
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl SmartEngine {
    /// Called by IBus for every key event in the focused application.
    /// Returns `true` to consume the key (prevent it reaching the app),
    /// `false` to pass it through.
    async fn process_key_event(
        &self,
        keyval: u32,
        _keycode: u32,
        modifier_state: u32,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) -> bool {
        // Ignore release events; let them through so the app stays consistent.
        if modifier_state & MOD_RELEASE != 0 {
            return false;
        }

        let mut st = self.state.lock().await;

        // ── Backspace: pop from preedit buffer ────────────────────────────────
        if keyval == KEY_BACKSPACE && !st.word_buffer.is_empty() {
            st.word_buffer.pop();
            st.refresh_suggestions(&self.completer.read().unwrap());
            let preedit = ibus_text(st.word_buffer.clone());
            let cursor = st.word_buffer.chars().count() as u32;
            let sugg = st.suggestions.clone();
            drop(st);
            let _ = Self::update_pre_edit_text(&ctxt, &preedit, cursor, true).await;
            if sugg.is_empty() {
                let _ = Self::hide_lookup_table(&ctxt).await;
            } else {
                let _ = Self::update_lookup_table(&ctxt, &ibus_lookup_table(&sugg), true).await;
            }
            return true;
        }

        // ── Tab: accept first suggestion ──────────────────────────────────────
        if keyval == KEY_TAB && !st.suggestions.is_empty() {
            let chosen = st.suggestions[0].clone();
            st.clear();
            drop(st);
            let _ = Self::hide_pre_edit_text(&ctxt).await;
            let _ = Self::hide_lookup_table(&ctxt).await;
            let _ = Self::commit_text(&ctxt, &ibus_text(chosen)).await;
            return true;
        }

        // ── 1–5: accept nth suggestion ────────────────────────────────────────
        if matches!(keyval, KEY_1..=KEY_5) && !st.suggestions.is_empty() {
            let idx = (keyval - KEY_1) as usize;
            if let Some(chosen) = st.suggestions.get(idx).cloned() {
                st.clear();
                drop(st);
                let _ = Self::hide_pre_edit_text(&ctxt).await;
                let _ = Self::hide_lookup_table(&ctxt).await;
                let _ = Self::commit_text(&ctxt, &ibus_text(chosen)).await;
                return true;
            }
        }

        // ── Space / Enter: commit buffer (with autocorrect) ───────────────────
        if keyval == KEY_SPACE || keyval == KEY_RETURN {
            let word = std::mem::take(&mut st.word_buffer);
            st.clear();
            drop(st);
            let _ = Self::hide_pre_edit_text(&ctxt).await;
            let _ = Self::hide_lookup_table(&ctxt).await;
            if !word.is_empty() {
                let committed = (self.autocorrect_fn)(&word).unwrap_or(word);
                let delimiter = if keyval == KEY_SPACE { " " } else { "\n" };
                let _ = Self::commit_text(
                    &ctxt,
                    &ibus_text(format!("{}{}", committed, delimiter)),
                )
                .await;
                return true;
            }
            return false;
        }

        // ── Navigation: commit pending preedit, pass key through ──────────────
        if matches!(
            keyval,
            KEY_ESCAPE
                | KEY_LEFT
                | KEY_RIGHT
                | KEY_UP
                | KEY_DOWN
                | KEY_HOME
                | KEY_END
                | KEY_DELETE
                | KEY_PAGE_UP
                | KEY_PAGE_DOWN
        ) {
            if !st.word_buffer.is_empty() {
                let word = std::mem::take(&mut st.word_buffer);
                st.clear();
                drop(st);
                let _ = Self::hide_pre_edit_text(&ctxt).await;
                let _ = Self::hide_lookup_table(&ctxt).await;
                let _ = Self::commit_text(&ctxt, &ibus_text(word)).await;
            }
            return false;
        }

        // ── Alphabetic / apostrophe: buffer the character ─────────────────────
        if let Some(ch) = keysym_to_char(keyval, modifier_state) {
            if ch.is_alphabetic() || ch == '\'' {
                st.word_buffer.push(ch);
                st.refresh_suggestions(&self.completer.read().unwrap());
                let preedit = ibus_text(st.word_buffer.clone());
                let cursor = st.word_buffer.chars().count() as u32;
                let sugg = st.suggestions.clone();
                drop(st);
                let _ = Self::update_pre_edit_text(&ctxt, &preedit, cursor, true).await;
                if sugg.is_empty() {
                    let _ = Self::hide_lookup_table(&ctxt).await;
                } else {
                    let _ =
                        Self::update_lookup_table(&ctxt, &ibus_lookup_table(&sugg), true).await;
                }
                return true;
            }
            // Non-alpha printable: flush any buffered preedit then pass through.
            if !st.word_buffer.is_empty() {
                let word = std::mem::take(&mut st.word_buffer);
                st.clear();
                drop(st);
                let _ = Self::hide_pre_edit_text(&ctxt).await;
                let _ = Self::hide_lookup_table(&ctxt).await;
                let _ = Self::commit_text(&ctxt, &ibus_text(word)).await;
            }
        }

        false
    }

    async fn focus_out(&self, #[zbus(signal_context)] ctxt: SignalContext<'_>) {
        let mut st = self.state.lock().await;
        st.clear();
        drop(st);
        let _ = Self::hide_pre_edit_text(&ctxt).await;
        let _ = Self::hide_lookup_table(&ctxt).await;
    }

    async fn reset(&self, #[zbus(signal_context)] ctxt: SignalContext<'_>) {
        let mut st = self.state.lock().await;
        st.clear();
        drop(st);
        let _ = Self::hide_pre_edit_text(&ctxt).await;
        let _ = Self::hide_lookup_table(&ctxt).await;
    }

    async fn disable(&self, #[zbus(signal_context)] ctxt: SignalContext<'_>) {
        let mut st = self.state.lock().await;
        st.clear();
        drop(st);
        let _ = Self::hide_pre_edit_text(&ctxt).await;
        let _ = Self::hide_lookup_table(&ctxt).await;
    }

    async fn candidate_clicked(
        &self,
        index: u32,
        _button: u32,
        _state: u32,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) {
        let mut st = self.state.lock().await;
        if let Some(chosen) = st.suggestions.get(index as usize).cloned() {
            st.clear();
            drop(st);
            let _ = Self::hide_pre_edit_text(&ctxt).await;
            let _ = Self::hide_lookup_table(&ctxt).await;
            let _ = Self::commit_text(&ctxt, &ibus_text(chosen)).await;
        }
    }

    fn enable(&self) {}
    fn focus_in(&self) {}
    fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}
    fn set_capabilities(&self, _caps: u32) {}
    fn page_up(&self) {}
    fn page_down(&self) {}
    fn cursor_up(&self) {}
    fn cursor_down(&self) {}
    fn property_activate(&self, _name: &str, _state: u32) {}
    fn property_show(&self, _name: &str) {}
    fn property_hide(&self, _name: &str) {}

    // ── Signals ───────────────────────────────────────────────────────────────

    #[zbus(signal)]
    async fn commit_text(ctxt: &SignalContext<'_>, text: &Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_pre_edit_text(
        ctxt: &SignalContext<'_>,
        text: &Value<'_>,
        cursor_pos: u32,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_pre_edit_text(ctxt: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_lookup_table(
        ctxt: &SignalContext<'_>,
        table: &Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_lookup_table(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
}

// ── Key decoding ──────────────────────────────────────────────────────────────

fn keysym_to_char(keyval: u32, modifier_state: u32) -> Option<char> {
    let shifted = modifier_state & MOD_SHIFT != 0;
    match keyval {
        // Lowercase keysyms (a–z); shift gives uppercase
        0x0061..=0x007a => {
            let ch = char::from_u32(keyval)?;
            Some(if shifted { ch.to_uppercase().next().unwrap_or(ch) } else { ch })
        }
        // Uppercase keysyms (A–Z) sent when CapsLock is active
        0x0041..=0x005a => {
            let lower = char::from_u32(keyval + 0x20)?;
            Some(if shifted { lower } else { char::from_u32(keyval)? })
        }
        0x0027 => Some('\''), // apostrophe
        _ => None,
    }
}

// ── SmartFactory: org.freedesktop.IBus.Factory ───────────────────────────────

struct SmartFactory {
    completer: Arc<RwLock<WordCompleter>>,
    autocorrect_fn: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
    counter: AtomicU32,
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl SmartFactory {
    /// IBus calls this when it wants to activate the SmartType engine.
    async fn create_engine(
        &self,
        engine_name: &str,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        if engine_name != "smarttype" {
            return Err(zbus::fdo::Error::Failed(format!(
                "unknown engine: {}",
                engine_name
            )));
        }
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        let path = OwnedObjectPath::try_from(format!(
            "/org/freedesktop/IBus/Engine/SmartType/{}",
            id
        ))
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let engine = SmartEngine::new(
            Arc::clone(&self.completer),
            Arc::clone(&self.autocorrect_fn),
        );
        server
            .at(path.clone(), engine)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        log::info!("Engine created at {}", path);
        Ok(path)
    }
}

// ── IBus socket discovery ─────────────────────────────────────────────────────

fn find_ibus_address() -> Result<String> {
    if let Ok(addr) = std::env::var("IBUS_ADDRESS") {
        if !addr.is_empty() {
            return Ok(addr);
        }
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", home));
    let machine_id = std::fs::read_to_string("/etc/machine-id")
        .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id"))
        .context("cannot read machine-id")?;
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    let disp_num = display.trim_start_matches(':').split('.').next().unwrap_or("0");
    let bus_file = format!(
        "{}/ibus/bus/{}-unix-{}",
        config_dir,
        machine_id.trim(),
        disp_num
    );
    let content = std::fs::read_to_string(&bus_file)
        .with_context(|| format!("cannot read IBus socket file: {}", bus_file))?;
    for line in content.lines() {
        if let Some(addr) = line.strip_prefix("IBUS_ADDRESS=") {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("IBUS_ADDRESS not found in {}", bus_file)
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let config = Config::load().unwrap_or_default();
    let ac_engine = Arc::new(RwLock::new(AutocorrectEngine::new(config)?));
    let autocorrect_fn: Arc<dyn Fn(&str) -> Option<String> + Send + Sync> = Arc::new({
        let ac = Arc::clone(&ac_engine);
        move |word: &str| ac.read().unwrap().correct_word(word)
    });
    let completer = Arc::new(RwLock::new(WordCompleter::new()));

    let ibus_addr =
        find_ibus_address().context("IBus not running — start with: ibus-daemon -drx")?;
    log::info!("Connecting to IBus at {}", ibus_addr);

    let factory = SmartFactory {
        completer,
        autocorrect_fn,
        counter: AtomicU32::new(0),
    };

    let _conn = zbus::ConnectionBuilder::address(ibus_addr.as_str())?
        .name("org.freedesktop.IBus.SmartType")?
        .serve_at("/org/freedesktop/IBus/Factory", factory)?
        .build()
        .await
        .context("Cannot connect to IBus")?;

    log::info!("SmartType IBus engine ready");
    std::future::pending::<()>().await;
    Ok(())
}
