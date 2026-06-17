/// Input hook: reads keyboard events via evdev, drives autocomplete + autocorrect,
/// types completions back through a raw uinput virtual keyboard (no libudev needed),
/// and sends suggestion lists to the popup via Unix socket.
use crate::autocomplete::WordCompleter;
use anyhow::{Context, Result};
use evdev::{Device, EventType, Key};
use futures::StreamExt;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::os::unix::io::AsRawFd as _;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex;

const POPUP_SOCKET: &str = "/tmp/smarttype-popup.sock";
const MAX_SUGGESTIONS: usize = 5;
const MIN_PREFIX_LEN: usize = 2;

// ── Raw uinput constants ──────────────────────────────────────────────────────

const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
const UI_DEV_SETUP: libc::c_ulong = 0x405C_5503;
const UI_SET_EVBIT: libc::c_ulong = 0x4004_5564;
const UI_SET_KEYBIT: libc::c_ulong = 0x4004_5565;

const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;
const SYN_REPORT: u16 = 0;
const KC_BACKSPACE: u16 = 14;
const KC_LEFTSHIFT: u16 = 42;

static ALL_KEYS: &[u16] = &[
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
    46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57,
];

#[repr(C)] struct InputId { bustype: u16, vendor: u16, product: u16, version: u16 }
#[repr(C)] struct UinputSetup { id: InputId, name: [u8; 80], ff_effects_max: u32 }
#[repr(C)] struct InputEvent { tv_sec: i64, tv_usec: i64, kind: u16, code: u16, value: i32 }

// ── VirtualKeyboard ───────────────────────────────────────────────────────────

pub struct VirtualKeyboard { dev: File, delay_ms: u64 }

impl VirtualKeyboard {
    pub fn new() -> Result<Self> {
        let dev = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uinput")
            .context("Cannot open /dev/uinput")?;
        let fd = dev.as_raw_fd();
        unsafe {
            if libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_int) < 0 {
                anyhow::bail!("UI_SET_EVBIT: {}", std::io::Error::last_os_error());
            }
            for &code in ALL_KEYS { libc::ioctl(fd, UI_SET_KEYBIT, code as libc::c_int); }
            let mut setup = UinputSetup {
                id: InputId { bustype: 0x03, vendor: 0, product: 0, version: 1 },
                name: [0u8; 80], ff_effects_max: 0,
            };
            let label = b"SmartType Virtual Keyboard";
            setup.name[..label.len()].copy_from_slice(label);
            if libc::ioctl(fd, UI_DEV_SETUP, &setup as *const UinputSetup) < 0 {
                anyhow::bail!("UI_DEV_SETUP: {}", std::io::Error::last_os_error());
            }
            if libc::ioctl(fd, UI_DEV_CREATE) < 0 {
                anyhow::bail!("UI_DEV_CREATE: {}", std::io::Error::last_os_error());
            }
        }
        thread::sleep(Duration::from_millis(150));
        log::info!("Virtual keyboard created via raw uinput");
        Ok(Self { dev, delay_ms: 2 })
    }

    fn write_event(&mut self, kind: u16, code: u16, value: i32) -> Result<()> {
        let ev = InputEvent { tv_sec: 0, tv_usec: 0, kind, code, value };
        let bytes = unsafe {
            std::slice::from_raw_parts(&ev as *const InputEvent as *const u8, std::mem::size_of::<InputEvent>())
        };
        self.dev.write_all(bytes).context("write input_event")
    }

    fn sync(&mut self) -> Result<()> { self.write_event(EV_SYN, SYN_REPORT, 0) }
    fn press(&mut self, c: u16) -> Result<()> { self.write_event(EV_KEY, c, 1)?; self.sync() }
    fn release(&mut self, c: u16) -> Result<()> { self.write_event(EV_KEY, c, 0)?; self.sync() }

    pub fn press_backspace(&mut self, n: usize) -> Result<()> {
        for _ in 0..n {
            self.press(KC_BACKSPACE)?; self.release(KC_BACKSPACE)?;
            thread::sleep(Duration::from_millis(self.delay_ms));
        }
        Ok(())
    }

    pub fn type_text(&mut self, text: &str) -> Result<()> {
        for ch in text.chars() {
            if let Some((code, shift)) = char_to_keycode(ch) {
                if shift { self.press(KC_LEFTSHIFT)?; }
                self.press(code)?; self.release(code)?;
                if shift { self.release(KC_LEFTSHIFT)?; }
                thread::sleep(Duration::from_millis(self.delay_ms));
            }
        }
        Ok(())
    }
}

impl Drop for VirtualKeyboard {
    fn drop(&mut self) { unsafe { libc::ioctl(self.dev.as_raw_fd(), UI_DEV_DESTROY); } }
}

// ── Shared hook state ─────────────────────────────────────────────────────────

struct HookState {
    /// Characters accumulated since last word boundary
    word_buffer: String,
    /// Suggestions currently displayed in the popup
    suggestions: Vec<String>,
    /// Set when the word before the last space looked like a typo.
    /// Tab/1-5 will replace (pending_word + space + key) with the chosen correction.
    pending_word: Option<String>,
    shift_pressed: bool,
    caps_lock: bool,
}

impl HookState {
    fn new() -> Self {
        Self { word_buffer: String::new(), suggestions: Vec::new(),
               pending_word: None, shift_pressed: false, caps_lock: false }
    }
}

// ── InputHook ─────────────────────────────────────────────────────────────────

pub struct InputHook {
    devices: Vec<Device>,
    completer: Arc<RwLock<WordCompleter>>,
    autocorrect_fn: Option<Arc<dyn Fn(&str) -> Option<String> + Send + Sync>>,
}

impl InputHook {
    pub fn new(completer: Arc<RwLock<WordCompleter>>) -> Result<Self> {
        Ok(Self { devices: Vec::new(), completer, autocorrect_fn: None })
    }

    pub fn set_autocorrect<F>(&mut self, f: F)
    where F: Fn(&str) -> Option<String> + Send + Sync + 'static {
        self.autocorrect_fn = Some(Arc::new(f));
    }

    pub async fn init(&mut self) -> Result<()> {
        self.devices = find_keyboard_devices()?;
        if self.devices.is_empty() {
            anyhow::bail!("No keyboard devices found — ensure your user is in the 'input' group");
        }
        log::info!("Found {} keyboard device(s)", self.devices.len());
        Ok(())
    }

    pub async fn start(self) -> Result<()> {
        let vkb: Option<Arc<Mutex<VirtualKeyboard>>> =
            match tokio::task::spawn_blocking(VirtualKeyboard::new).await {
                Ok(Ok(kb)) => { log::info!("Virtual keyboard ready"); Some(Arc::new(Mutex::new(kb))) }
                Ok(Err(e)) => { log::warn!("Virtual keyboard unavailable: {}", e); None }
                Err(e)     => { log::warn!("spawn_blocking error: {}", e); None }
            };

        let completer = self.completer;
        let autocorrect_fn = self.autocorrect_fn;

        loop {
            let devices = match find_keyboard_devices() {
                Ok(d) if !d.is_empty() => d,
                Ok(_) => {
                    log::warn!("No keyboards — retrying in 2s");
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                }
                Err(e) => {
                    log::warn!("Device discovery: {} — retrying in 2s", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                }
            };

            let state = Arc::new(Mutex::new(HookState::new()));
            let mut handles = Vec::new();

            for device in devices {
                let stream = match device.into_event_stream() {
                    Ok(s) => s,
                    Err(e) => { log::warn!("Cannot open stream: {}", e); continue; }
                };
                let state = Arc::clone(&state);
                let completer = Arc::clone(&completer);
                let vkb = vkb.clone();
                let autocorrect_fn = autocorrect_fn.clone();
                handles.push(tokio::spawn(async move {
                    run_stream(stream, state, completer, vkb, autocorrect_fn).await;
                }));
            }

            if handles.is_empty() {
                log::warn!("No usable streams — retrying in 2s");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }

            futures::future::join_all(handles).await;
            log::warn!("All streams ended — rediscovering in 2s");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
}

// ── Per-device event loop ─────────────────────────────────────────────────────

async fn run_stream(
    mut stream: evdev::EventStream,
    state: Arc<Mutex<HookState>>,
    completer: Arc<RwLock<WordCompleter>>,
    vkb: Option<Arc<Mutex<VirtualKeyboard>>>,
    autocorrect_fn: Option<Arc<dyn Fn(&str) -> Option<String> + Send + Sync>>,
) {
    log::info!("Stream started: {:?}", stream.device().name().unwrap_or("?"));
    loop {
        match stream.next().await {
            Some(Ok(event)) => {
                if event.event_type() != EventType::KEY { continue; }
                on_key(Key::new(event.code()), event.value(),
                       &state, &completer, &vkb, &autocorrect_fn).await;
            }
            Some(Err(e)) => {
                let fatal = e.raw_os_error()
                    .map(|n| n == libc::ENODEV || n == libc::ENXIO)
                    .unwrap_or(false);
                if fatal { log::warn!("Device removed: {} — stopping stream", e); break; }
                log::warn!("Stream error: {} — retrying", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            None => { log::warn!("Stream ended (device gone?)"); break; }
        }
    }
}

async fn on_key(
    key: Key,
    value: i32,
    state: &Arc<Mutex<HookState>>,
    completer: &Arc<RwLock<WordCompleter>>,
    vkb: &Option<Arc<Mutex<VirtualKeyboard>>>,
    autocorrect_fn: &Option<Arc<dyn Fn(&str) -> Option<String> + Send + Sync>>,
) {
    let mut s = state.lock().await;

    if value == 0 {
        if matches!(key, Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT) { s.shift_pressed = false; }
        return;
    }

    match key {
        Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT => { s.shift_pressed = true; }
        Key::KEY_CAPSLOCK => { s.caps_lock = !s.caps_lock; }

        // ── Backspace ─────────────────────────────────────────────────────────
        Key::KEY_BACKSPACE => {
            if let Some(pending_str) = s.pending_word.take() {
                // Backspace erased the space after a possibly-typo'd word.
                // Restore the word into buffer so the user can keep editing it.
                s.word_buffer = pending_str;
                refresh_suggestions(&mut s, completer);
                let sugg = s.suggestions.clone();
                drop(s);
                send_to_popup(&sugg);
            } else {
                s.word_buffer.pop();
                refresh_suggestions(&mut s, completer);
                let sugg = s.suggestions.clone();
                drop(s);
                send_to_popup(&sugg);
            }
        }

        // ── Tab → accept first suggestion ─────────────────────────────────────
        Key::KEY_TAB if !s.suggestions.is_empty() => {
            let chosen = s.suggestions[0].clone();
            let pending = s.pending_word.take();
            let prefix_len = s.word_buffer.chars().count();
            s.word_buffer.clear();
            s.suggestions.clear();
            drop(s);
            send_to_popup(&[]);
            do_learn(completer, &chosen);
            if let Some(p) = pending {
                // erase: original word + space + Tab, then type correction + space
                type_replacing(vkb, p.chars().count() + 2, &format!("{} ", chosen)).await;
            } else {
                // erase: typed prefix + Tab, then type completion
                type_replacing(vkb, prefix_len + 1, &chosen).await;
            }
        }

        // ── 1-5 → accept nth suggestion ───────────────────────────────────────
        Key::KEY_1 | Key::KEY_2 | Key::KEY_3 | Key::KEY_4 | Key::KEY_5
            if !s.suggestions.is_empty() =>
        {
            let idx = match key {
                Key::KEY_1 => 0, Key::KEY_2 => 1, Key::KEY_3 => 2,
                Key::KEY_4 => 3, _           => 4,
            };
            if let Some(chosen) = s.suggestions.get(idx).cloned() {
                let pending = s.pending_word.take();
                let prefix_len = s.word_buffer.chars().count();
                s.word_buffer.clear();
                s.suggestions.clear();
                drop(s);
                send_to_popup(&[]);
                do_learn(completer, &chosen);
                if let Some(p) = pending {
                    type_replacing(vkb, p.chars().count() + 2, &format!("{} ", chosen)).await;
                } else {
                    type_replacing(vkb, prefix_len + 1, &chosen).await;
                }
            }
        }

        // ── Word boundary ─────────────────────────────────────────────────────
        Key::KEY_SPACE | Key::KEY_ENTER | Key::KEY_LINEFEED => {
            let word = s.word_buffer.clone();
            s.pending_word = None;
            s.word_buffer.clear();
            s.suggestions.clear();
            drop(s);

            if word.is_empty() { send_to_popup(&[]); return; }

            // Gently boost known words the user typed (confirms usage)
            if completer.read().unwrap().contains(&word) {
                do_learn(completer, &word);
            }

            // Explicit autocorrect rules take priority
            if let Some(ref f) = autocorrect_fn {
                if let Some(correction) = f(&word) {
                    log::info!("Autocorrect: {} → {}", word, correction);
                    type_replacing(vkb, word.chars().count(), &correction).await;
                    send_to_popup(&[]);
                    return;
                }
            }

            // If the word isn't in the dictionary, show typo corrections
            let corrections = completer.read().unwrap().correct(&word, MAX_SUGGESTIONS);
            if !corrections.is_empty() {
                let mut st = state.lock().await;
                st.suggestions = corrections.clone();
                st.pending_word = Some(word);
                drop(st);
                send_to_popup(&corrections);
            } else {
                send_to_popup(&[]);
            }
        }

        // ── Navigation / function keys → reset context ────────────────────────
        Key::KEY_ESC
        | Key::KEY_LEFT | Key::KEY_RIGHT | Key::KEY_UP | Key::KEY_DOWN
        | Key::KEY_HOME | Key::KEY_END | Key::KEY_DELETE
        | Key::KEY_PAGEUP | Key::KEY_PAGEDOWN => {
            s.pending_word = None;
            s.word_buffer.clear();
            s.suggestions.clear();
            drop(s);
            send_to_popup(&[]);
        }

        // ── Alphabetic / other ────────────────────────────────────────────────
        _ => {
            if let Some(base) = key_to_base_char(key) {
                // First letter of a new word clears any pending post-space correction
                if s.pending_word.is_some() {
                    s.pending_word = None;
                    s.word_buffer.clear();
                    s.suggestions.clear();
                }
                s.word_buffer.push(base);
                refresh_suggestions(&mut s, completer);
                let sugg = s.suggestions.clone();
                drop(s);
                send_to_popup(&sugg);
            } else {
                s.pending_word = None;
                s.word_buffer.clear();
                s.suggestions.clear();
                drop(s);
                send_to_popup(&[]);
            }
        }
    }
}

fn refresh_suggestions(s: &mut HookState, completer: &Arc<RwLock<WordCompleter>>) {
    s.suggestions = if s.word_buffer.len() >= MIN_PREFIX_LEN {
        completer.read().unwrap().suggest(&s.word_buffer, MAX_SUGGESTIONS)
    } else {
        Vec::new()
    };
}

/// Learn a word and persist the learned map asynchronously (fire-and-forget).
fn do_learn(completer: &Arc<RwLock<WordCompleter>>, word: &str) {
    if let Some((json, path)) = completer.write().unwrap().learn(word) {
        tokio::task::spawn_blocking(move || {
            if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
            let _ = std::fs::write(&path, json);
        });
    }
}

// ── Typing via virtual keyboard ───────────────────────────────────────────────

async fn type_replacing(vkb: &Option<Arc<Mutex<VirtualKeyboard>>>, backspaces: usize, text: &str) {
    let Some(vkb) = vkb else { return; };
    let text = text.to_string();
    let vkb = Arc::clone(vkb);
    match tokio::task::spawn_blocking(move || {
        let mut kb = vkb.blocking_lock();
        kb.press_backspace(backspaces)?;
        kb.type_text(&text)
    }).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => log::error!("type_replacing: {}", e),
        Err(e)     => log::error!("type_replacing spawn: {}", e),
    }
}

// ── Popup IPC ─────────────────────────────────────────────────────────────────

fn send_to_popup(suggestions: &[String]) {
    let msg = if suggestions.is_empty() {
        "\n".to_string()
    } else {
        format!("{}\n", suggestions.join(","))
    };
    tokio::task::spawn_blocking(move || {
        if let Ok(mut s) = UnixStream::connect(POPUP_SOCKET) {
            s.set_write_timeout(Some(Duration::from_millis(20))).ok();
            let _ = s.write_all(msg.as_bytes());
        }
    });
}

// ── Device discovery ──────────────────────────────────────────────────────────

fn find_keyboard_devices() -> Result<Vec<Device>> {
    let mut keyboards = Vec::new();
    for entry in std::fs::read_dir("/dev/input").context("read /dev/input")? {
        let Ok(e) = entry else { continue };
        if !e.file_name().to_string_lossy().starts_with("event") { continue; }
        let Ok(dev) = Device::open(e.path()) else { continue };
        if dev.name() == Some("SmartType Virtual Keyboard") { continue; }
        if dev.supported_events().contains(EventType::KEY) {
            if let Some(keys) = dev.supported_keys() {
                if keys.contains(Key::KEY_A) && keys.contains(Key::KEY_Z) {
                    log::info!("Keyboard: {:?} ({})", e.path(), dev.name().unwrap_or("?"));
                    keyboards.push(dev);
                }
            }
        }
    }
    Ok(keyboards)
}

// ── Key tables ────────────────────────────────────────────────────────────────

fn key_to_base_char(key: Key) -> Option<char> {
    match key {
        Key::KEY_A => Some('a'), Key::KEY_B => Some('b'), Key::KEY_C => Some('c'),
        Key::KEY_D => Some('d'), Key::KEY_E => Some('e'), Key::KEY_F => Some('f'),
        Key::KEY_G => Some('g'), Key::KEY_H => Some('h'), Key::KEY_I => Some('i'),
        Key::KEY_J => Some('j'), Key::KEY_K => Some('k'), Key::KEY_L => Some('l'),
        Key::KEY_M => Some('m'), Key::KEY_N => Some('n'), Key::KEY_O => Some('o'),
        Key::KEY_P => Some('p'), Key::KEY_Q => Some('q'), Key::KEY_R => Some('r'),
        Key::KEY_S => Some('s'), Key::KEY_T => Some('t'), Key::KEY_U => Some('u'),
        Key::KEY_V => Some('v'), Key::KEY_W => Some('w'), Key::KEY_X => Some('x'),
        Key::KEY_Y => Some('y'), Key::KEY_Z => Some('z'),
        Key::KEY_APOSTROPHE => Some('\''),
        _ => None,
    }
}

fn char_to_keycode(ch: char) -> Option<(u16, bool)> {
    Some(match ch {
        'a'|'A' => (30, ch.is_uppercase()), 'b'|'B' => (48, ch.is_uppercase()),
        'c'|'C' => (46, ch.is_uppercase()), 'd'|'D' => (32, ch.is_uppercase()),
        'e'|'E' => (18, ch.is_uppercase()), 'f'|'F' => (33, ch.is_uppercase()),
        'g'|'G' => (34, ch.is_uppercase()), 'h'|'H' => (35, ch.is_uppercase()),
        'i'|'I' => (23, ch.is_uppercase()), 'j'|'J' => (36, ch.is_uppercase()),
        'k'|'K' => (37, ch.is_uppercase()), 'l'|'L' => (38, ch.is_uppercase()),
        'm'|'M' => (50, ch.is_uppercase()), 'n'|'N' => (49, ch.is_uppercase()),
        'o'|'O' => (24, ch.is_uppercase()), 'p'|'P' => (25, ch.is_uppercase()),
        'q'|'Q' => (16, ch.is_uppercase()), 'r'|'R' => (19, ch.is_uppercase()),
        's'|'S' => (31, ch.is_uppercase()), 't'|'T' => (20, ch.is_uppercase()),
        'u'|'U' => (22, ch.is_uppercase()), 'v'|'V' => (47, ch.is_uppercase()),
        'w'|'W' => (17, ch.is_uppercase()), 'x'|'X' => (45, ch.is_uppercase()),
        'y'|'Y' => (21, ch.is_uppercase()), 'z'|'Z' => (44, ch.is_uppercase()),
        '1' => (2, false),  '!' => (2, true),  '2' => (3, false),  '@' => (3, true),
        '3' => (4, false),  '#' => (4, true),  '4' => (5, false),  '$' => (5, true),
        '5' => (6, false),  '%' => (6, true),  '6' => (7, false),  '^' => (7, true),
        '7' => (8, false),  '&' => (8, true),  '8' => (9, false),  '*' => (9, true),
        '9' => (10, false), '(' => (10, true), '0' => (11, false), ')' => (11, true),
        '-' => (12, false), '_' => (12, true), '=' => (13, false), '+' => (13, true),
        ' ' => (57, false),
        '\'' => (40, false), '"' => (40, true),
        ',' => (51, false),  '<' => (51, true),
        '.' => (52, false),  '>' => (52, true),
        '/' => (53, false),  '?' => (53, true),
        ';' => (39, false),  ':' => (39, true),
        '[' => (26, false),  '{' => (26, true),
        ']' => (27, false),  '}' => (27, true),
        '\\' => (43, false), '|' => (43, true),
        '`' => (41, false),  '~' => (41, true),
        _ => return None,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_letter_keycodes() {
        for ch in 'a'..='z' {
            let (code, shift) = char_to_keycode(ch).unwrap();
            assert!(!shift);
            let (code2, shift2) = char_to_keycode(ch.to_ascii_uppercase()).unwrap();
            assert_eq!(code, code2);
            assert!(shift2);
        }
    }
}
