/// SmartType popup — pure-Rust X11 suggestion chip bar.
///
/// Socket protocol (from hook):
///   "word1,word2,word3\n"  → show suggestions
///   "\n"                    → hide
///
/// The window floats above the mouse cursor using `override_redirect` so it
/// never steals focus. Suggestions are colour-coded by priority.
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_FROM_PARENT;

const SOCKET_PATH: &str = "/tmp/smarttype-popup.sock";
const BAR_H: u16 = 34;
const CHIP_PAD: i16 = 18; // horizontal gap between chips
const LEFT_PAD: i16 = 10; // left margin inside window
const CHIP_VPAD: i16 = 3; // vertical padding inside chip highlight box

// ── Catppuccin Mocha palette ──────────────────────────────────────────────────
const BG: u32 = 0x1e1e2e;       // base
const BG_CHIP1: u32 = 0x313244; // surface0 — highlight box for best suggestion
const BORDER_COL: u32 = 0x45475a; // surface1 — window border
const LABEL_COL: u32 = 0x585b70; // overlay0 — dimmed key labels

/// Suggestion word colours in priority order (Tab = index 0, best match)
const WORD_COLORS: [u32; 5] = [
    0xf9e2af, // peach/gold — best (Tab)
    0xa6e3a1, // green      — 2nd
    0x89b4fa, // blue       — 3rd
    0xcba6f7, // lavender   — 4th
    0x89dceb, // sky        — 5th
];

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct PopupState {
    suggestions: Vec<String>,
    dirty: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    env_logger::init();

    let state: Arc<Mutex<PopupState>> = Arc::new(Mutex::new(PopupState::default()));
    {
        let state = Arc::clone(&state);
        thread::spawn(move || socket_listener(state));
    }
    x11_event_loop(state)
}

// ── Socket listener (background thread) ──────────────────────────────────────

fn socket_listener(state: Arc<Mutex<PopupState>>) {
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => { log::error!("Cannot bind {}: {}", SOCKET_PATH, e); return; }
    };
    let _ = std::fs::set_permissions(
        SOCKET_PATH,
        std::os::unix::fs::PermissionsExt::from_mode(0o666),
    );
    log::info!("Popup listening on {}", SOCKET_PATH);

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            let suggestions: Vec<String> = line
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let mut s = state.lock().unwrap();
            s.suggestions = suggestions;
            s.dirty = true;
        }
    }
}

// ── X11 event loop ────────────────────────────────────────────────────────────

fn x11_event_loop(state: Arc<Mutex<PopupState>>) -> Result<()> {
    let (conn, screen_num) =
        RustConnection::connect(None).context("Cannot connect to X11 — is DISPLAY set?")?;
    let screen = &conn.setup().roots[screen_num];
    let screen_w = screen.width_in_pixels;
    let screen_h = screen.height_in_pixels;
    let root = screen.root;

    // Create popup window: starts tiny and unmapped; repositioned before each show
    let win = conn.generate_id()?;
    conn.create_window(
        COPY_FROM_PARENT as u8,
        win,
        root,
        0, 0,          // x, y — overwritten before first show
        1, BAR_H,      // width (1 = placeholder), height
        1,             // border width
        WindowClass::INPUT_OUTPUT,
        COPY_FROM_PARENT,
        &CreateWindowAux::new()
            .background_pixel(BG)
            .border_pixel(BORDER_COL)
            .override_redirect(1u32) // never managed / never steals focus
            .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY),
    )?;

    set_atom_property(&conn, win, "_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE_DOCK")?;
    set_atom_property(&conn, win, "_NET_WM_STATE", "_NET_WM_STATE_ABOVE")?;

    // Single GC — we change foreground before each segment
    let gc = conn.generate_id()?;
    conn.create_gc(
        gc, win,
        &CreateGCAux::new()
            .foreground(0xcdd6f4)
            .background(BG)
            .graphics_exposures(0),
    )?;

    // Load fixed font and query its metrics
    let font = conn.generate_id()?;
    conn.open_font(font, b"fixed")?;
    conn.change_gc(gc, &ChangeGCAux::new().font(font))?;
    let finfo = conn.query_font(font)?.reply()?;
    let char_w  = finfo.max_bounds.character_width.max(1) as i16;
    let ascent  = finfo.max_bounds.ascent as i16;
    let descent = finfo.max_bounds.descent as i16;
    // Baseline Y so text is vertically centred inside BAR_H
    let text_y = (BAR_H as i16 - (ascent + descent)) / 2 + ascent;

    conn.flush()?;

    let mut visible = false;

    loop {
        // Drain pending X events
        while let Some(event) = conn.poll_for_event()? {
            if let Event::Expose(e) = event {
                if e.count == 0 {
                    let s = state.lock().unwrap();
                    if !s.suggestions.is_empty() {
                        if let Err(e) = render(&conn, win, gc, char_w, text_y, &s.suggestions) {
                            log::warn!("Expose redraw: {}", e);
                        }
                    }
                }
            }
        }

        // Pick up state changes pushed by the socket thread
        let (suggestions, dirty) = {
            let mut s = state.lock().unwrap();
            let d = s.dirty;
            s.dirty = false;
            (s.suggestions.clone(), d)
        };

        if dirty {
            if suggestions.is_empty() {
                if visible {
                    conn.unmap_window(win)?;
                    conn.flush()?;
                    visible = false;
                }
            } else {
                // Reposition BEFORE mapping so the window never appears in the wrong place
                if let Err(e) = reposition(&conn, win, root, screen_w, screen_h, char_w, &suggestions) {
                    log::warn!("reposition: {}", e);
                }
                if !visible {
                    conn.map_window(win)?;
                    conn.configure_window(
                        win,
                        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
                    )?;
                    visible = true;
                }
                if let Err(e) = render(&conn, win, gc, char_w, text_y, &suggestions) {
                    log::warn!("render: {}", e);
                }
                conn.flush()?;
            }
        }

        thread::sleep(std::time::Duration::from_millis(33));
    }
}

// ── Window positioning ────────────────────────────────────────────────────────

/// Query the mouse-pointer position and move the popup window so it floats
/// just above the cursor (or below if near the top of the screen).
fn reposition(
    conn: &RustConnection,
    win: Window,
    root: Window,
    screen_w: u16,
    screen_h: u16,
    char_w: i16,
    suggestions: &[String],
) -> Result<()> {
    let ptr = conn.query_pointer(root)?.reply()?;

    let popup_w = chip_row_width(char_w, suggestions).min(screen_w - 4);

    // Centre horizontally on cursor, clamp to screen edges
    let win_x = ((ptr.root_x as i32) - (popup_w as i32 / 2))
        .max(2)
        .min(screen_w as i32 - popup_w as i32 - 2);

    // Prefer above cursor; flip below if cursor is near the top
    let gap: i32 = 14;
    let win_y = if ptr.root_y as i32 > BAR_H as i32 + gap + 10 {
        (ptr.root_y as i32 - BAR_H as i32 - gap).max(2)
    } else {
        (ptr.root_y as i32 + gap + 8).min(screen_h as i32 - BAR_H as i32 - 2)
    };

    conn.configure_window(
        win,
        &ConfigureWindowAux::new()
            .x(win_x)
            .y(win_y)
            .width(popup_w as u32)
            .height(BAR_H as u32),
    )?;
    Ok(())
}

/// Total pixel width needed to display all suggestion chips.
fn chip_row_width(char_w: i16, suggestions: &[String]) -> u16 {
    let mut w: i16 = LEFT_PAD * 2;
    for (i, word) in suggestions.iter().enumerate().take(5) {
        let key_chars: i16 = if i == 0 { 4 } else { 2 }; // "Tab:" vs "2:"
        w += (key_chars + word.len() as i16) * char_w + CHIP_PAD;
    }
    w.max(160) as u16
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn render(
    conn: &RustConnection,
    win: Window,
    gc: Gcontext,
    char_w: i16,
    text_y: i16,
    suggestions: &[String],
) -> Result<()> {
    // Erase the whole window (width=0,height=0 means full window in XClearArea)
    conn.clear_area(false, win, 0, 0, 0, 0)?;

    let mut x: i16 = LEFT_PAD;

    for (i, word) in suggestions.iter().enumerate().take(5) {
        let key_str: &str = match i {
            0 => "Tab:", 1 => "2:", 2 => "3:", 3 => "4:", _ => "5:",
        };
        let word_color = WORD_COLORS[i];
        let chip_w = ((key_str.len() + word.len()) as i16 * char_w + 8) as u16;

        // ── Highlight box for the best suggestion ─────────────────────────────
        if i == 0 {
            conn.change_gc(gc, &ChangeGCAux::new().foreground(BG_CHIP1))?;
            conn.poly_fill_rectangle(
                win, gc,
                &[Rectangle {
                    x: x - 4,
                    y: CHIP_VPAD,
                    width: chip_w,
                    height: BAR_H - (CHIP_VPAD as u16) * 2,
                }],
            )?;
        }

        // ── Key label (dimmed) ────────────────────────────────────────────────
        conn.change_gc(gc, &ChangeGCAux::new().foreground(LABEL_COL))?;
        xtext(conn, win, gc, x, text_y, key_str)?;
        x += (key_str.len() as i16) * char_w;

        // ── Suggestion word (priority colour) ─────────────────────────────────
        conn.change_gc(gc, &ChangeGCAux::new().foreground(word_color))?;
        xtext(conn, win, gc, x, text_y, word)?;
        x += (word.len() as i16) * char_w + CHIP_PAD;
    }

    Ok(())
}

/// Draw a string using image_text8, respecting the 254-byte limit per call.
fn xtext(conn: &RustConnection, win: Window, gc: Gcontext, x: i16, y: i16, s: &str) -> Result<()> {
    let mut cx = x;
    for chunk in s.as_bytes().chunks(254) {
        conn.image_text8(win, gc, cx, y, chunk).context("image_text8")?;
        cx += chunk.len() as i16; // advance for multi-chunk strings
    }
    Ok(())
}

// ── EWMH atom helpers ─────────────────────────────────────────────────────────

fn set_atom_property(
    conn: &RustConnection,
    win: Window,
    property: &str,
    value: &str,
) -> Result<()> {
    let prop = conn.intern_atom(false, property.as_bytes())?.reply()?.atom;
    let val  = conn.intern_atom(false, value.as_bytes())?.reply()?.atom;
    conn.change_property32(PropMode::REPLACE, win, prop, AtomEnum::ATOM, &[val])?;
    Ok(())
}
