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
/// Ctrl / Alt(Mod1) / Super(Mod4): when any is held the key is a shortcut
/// (Ctrl+C, Alt+Tab, Super+…), never text — pass it straight through.
const MOD_CONTROL: u32 = 1 << 2; // 0x04
const MOD_ALT: u32 = 1 << 3; // 0x08
const MOD_SUPER: u32 = 1 << 6; // 0x40
const MOD_COMBO: u32 = MOD_CONTROL | MOD_ALT | MOD_SUPER;
/// IBus sets bit 30 on key-release events.
const MOD_RELEASE: u32 = 1 << 30;

const MIN_PREFIX_LEN: usize = 2;
const MAX_SUGGESTIONS: usize = 5;

/// IBusPreeditFocusMode: commit the preedit when focus is lost (vs. clear).
const IBUS_ENGINE_PREEDIT_COMMIT: u32 = 1;

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

// IMPORTANT: build these with `add_field` for concrete-typed fields (string,
// uint, bool, int) and `append_field` for the dynamically-typed Value fields
// (dict, array, nested variant). Passing a `Value` to `add_field` would type the
// field as a variant `v`, producing e.g. `(vvvv)` instead of `(sa{sv}sv)`; IBus
// then fails to deserialize ("format string '&s' ... given value has type 'v'")
// and silently drops the signal — no preedit/commit ever appears.

/// Builds an IBusAttrList variant (no attributes) → `(sa{sv}av)`.
fn ibus_attr_list() -> Value<'static> {
    Value::Structure(
        StructureBuilder::new()
            .add_field("IBusAttrList")
            .append_field(empty_sv_dict())
            .append_field(Value::Array(Array::new(sig("v"))))
            .build(),
    )
}

/// Builds an IBusText variant wrapping `text` → `(sa{sv}sv)`.
fn ibus_text(text: String) -> Value<'static> {
    Value::Structure(
        StructureBuilder::new()
            .add_field("IBusText")
            .append_field(empty_sv_dict())
            .add_field(text)
            .append_field(Value::Value(Box::new(ibus_attr_list())))
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
            .add_field("IBusLookupTable")
            .append_field(empty_sv_dict())
            .add_field(MAX_SUGGESTIONS as u32)              // page_size
            .add_field(0u32)                                // cursor_pos
            .add_field(true)                                // cursor_visible
            .add_field(false)                               // round
            .add_field(-1i32)                               // orientation: system default
            .append_field(Value::Array(cands))              // candidates
            .append_field(Value::Array(Array::new(sig("v")))) // labels (empty)
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
            completer
                .suggest(&self.word_buffer, MAX_SUGGESTIONS)
                .into_iter()
                .map(|s| match_case(&self.word_buffer, &s))
                .collect()
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
        log::debug!(
            "process_key_event keyval=0x{:x} keycode={} state=0x{:x}",
            keyval,
            _keycode,
            modifier_state
        );
        // Ignore release events; let them through so the app stays consistent.
        if modifier_state & MOD_RELEASE != 0 {
            return false;
        }

        // Ctrl/Alt/Super combo → keyboard shortcut. Flush any pending preedit so
        // the typed word isn't lost, then pass the key through to the app.
        if modifier_state & MOD_COMBO != 0 {
            let mut st = self.state.lock().await;
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

        let mut st = self.state.lock().await;

        // ── Backspace: pop from preedit buffer ────────────────────────────────
        if keyval == KEY_BACKSPACE && !st.word_buffer.is_empty() {
            st.word_buffer.pop();
            st.refresh_suggestions(&self.completer.read().unwrap());
            let preedit = ibus_text(st.word_buffer.clone());
            let cursor = st.word_buffer.chars().count() as u32;
            let sugg = st.suggestions.clone();
            drop(st);
            let _ = Self::update_pre_edit_text(
                &ctxt,
                &preedit,
                cursor,
                true,
                IBUS_ENGINE_PREEDIT_COMMIT,
            )
            .await;
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
                let committed = match (self.autocorrect_fn)(&word) {
                    Some(c) => match_case(&word, &c),
                    None => word,
                };
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
                if let Err(e) = Self::update_pre_edit_text(
                    &ctxt,
                    &preedit,
                    cursor,
                    true,
                    IBUS_ENGINE_PREEDIT_COMMIT,
                )
                .await
                {
                    log::error!("update_pre_edit_text failed: {}", e);
                }
                if sugg.is_empty() {
                    let _ = Self::hide_lookup_table(&ctxt).await;
                } else if let Err(e) =
                    Self::update_lookup_table(&ctxt, &ibus_lookup_table(&sugg), true).await
                {
                    log::error!("update_lookup_table failed: {}", e);
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

    // NOTE: IBus uses "Preedit" as a single word — the D-Bus signal is
    // `UpdatePreeditText`, not `UpdatePreEditText`. zbus would derive the wrong
    // PascalCase name from `update_pre_edit_text`, so pin the name explicitly.
    // IBus 1.5.x adds a 4th `mode` arg (u). GDBus validates incoming signals
    // against the introspected interface, so a 3-arg signal is dropped silently.
    #[zbus(signal, name = "UpdatePreeditText")]
    async fn update_pre_edit_text(
        ctxt: &SignalContext<'_>,
        text: &Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal, name = "HidePreeditText")]
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

/// Recase a completer suggestion (always lowercase) to match how the user typed
/// the prefix: "HEL" → "HELLO", "Hel" → "Hello", "hel" → "hello".
fn match_case(prefix: &str, suggestion: &str) -> String {
    if !prefix.chars().any(|c| c.is_uppercase()) {
        return suggestion.to_string();
    }
    if prefix.chars().count() > 1 && prefix.chars().all(|c| c.is_uppercase()) {
        return suggestion.to_uppercase();
    }
    // Leading capital only.
    let mut out = String::with_capacity(suggestion.len());
    let mut chars = suggestion.chars();
    if let Some(first) = chars.next() {
        out.extend(first.to_uppercase());
        out.extend(chars);
    }
    out
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

/// Collect every candidate IBus address, newest socket first.
///
/// We can't rely on `WAYLAND_DISPLAY`/`DISPLAY` being set: when launched from a
/// systemd user service the engine often inherits neither, so picking a single
/// file by env var lands on a stale socket (Connection refused). Instead we read
/// every address file IBus has written and order them by mtime, newest first —
/// the live session's socket is always the most recently written one.
fn find_ibus_addresses() -> Result<Vec<String>> {
    if let Ok(addr) = std::env::var("IBUS_ADDRESS") {
        if !addr.is_empty() {
            return Ok(vec![addr]);
        }
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", home));
    let bus_dir = format!("{}/ibus/bus", config_dir);

    let mut files: Vec<(std::time::SystemTime, std::path::PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(&bus_dir)
        .with_context(|| format!("cannot read IBus bus dir {}", bus_dir))?
        .flatten()
    {
        let path = entry.path();
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        files.push((mtime, path));
    }
    // Newest file first — that's the live session's socket.
    files.sort_by(|a, b| b.0.cmp(&a.0));

    let mut addrs = Vec::new();
    for (_, path) in &files {
        let Ok(content) = std::fs::read_to_string(path) else { continue };
        for line in content.lines() {
            if let Some(addr) = line.strip_prefix("IBUS_ADDRESS=") {
                log::debug!("IBus address candidate from {}", path.display());
                addrs.push(addr.to_string());
            }
        }
    }
    if addrs.is_empty() {
        anyhow::bail!("no IBUS_ADDRESS found in {}", bus_dir);
    }
    Ok(addrs)
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

    let addrs =
        find_ibus_addresses().context("IBus not running — start with: ibus-daemon -drx")?;

    // Try each candidate socket; stale ones (e.g. an old X11 session left over
    // in the bus dir) refuse the connection, so fall through to the next.
    let mut conn = None;
    let mut last_err = None;
    for addr in &addrs {
        log::info!("Connecting to IBus at {}", addr);
        let factory = SmartFactory {
            completer: Arc::clone(&completer),
            autocorrect_fn: Arc::clone(&autocorrect_fn),
            counter: AtomicU32::new(0),
        };
        match zbus::ConnectionBuilder::address(addr.as_str())
            .and_then(|b| b.name("org.freedesktop.IBus.SmartType"))
            .and_then(|b| {
                b.serve_at("/org/freedesktop/IBus/Factory", factory)
            }) {
            Ok(builder) => match builder.build().await {
                Ok(c) => {
                    conn = Some(c);
                    break;
                }
                Err(e) => {
                    log::warn!("Failed to connect to {}: {}", addr, e);
                    last_err = Some(e);
                }
            },
            Err(e) => {
                log::warn!("Bad IBus address {}: {}", addr, e);
                last_err = Some(e);
            }
        }
    }
    let _conn = conn.ok_or_else(|| {
        anyhow::anyhow!(
            "Cannot connect to IBus on any known socket: {}",
            last_err
                .map(|e| e.to_string())
                .unwrap_or_else(|| "no candidates".into())
        )
    })?;

    log::info!("SmartType IBus engine ready");
    std::future::pending::<()>().await;
    Ok(())
}
