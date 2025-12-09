use anyhow::{Result, Context};
use evdev::{Device, EventType, InputEvent, Key};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use uinput::event::keyboard;
use std::sync::LazyLock;

/// Global virtual keyboard instance for text replacement
static VIRTUAL_KEYBOARD: LazyLock<Arc<Mutex<Option<VirtualKeyboard>>>> = 
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Input hook for system-wide keyboard interception
pub struct InputHook {
    devices: Vec<Device>,
    word_buffer: Arc<RwLock<WordBuffer>>,
    callback: Option<Arc<dyn Fn(String) -> Option<String> + Send + Sync>>,
}

/// Buffer for collecting typed words
struct WordBuffer {
    buffer: String,
    last_keypress: Instant,
    timeout: Duration,
}

impl WordBuffer {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            last_keypress: Instant::now(),
            timeout: Duration::from_secs(2),
        }
    }

    fn add_char(&mut self, ch: char) {
        // Reset buffer if too much time has passed
        if self.last_keypress.elapsed() > self.timeout {
            self.buffer.clear();
        }

        self.buffer.push(ch);
        self.last_keypress = Instant::now();
    }

    fn add_key(&mut self, key: Key) {
        match key {
            Key::KEY_BACKSPACE => {
                self.buffer.pop();
            }
            Key::KEY_SPACE => {
                // Word boundary - time to check for correction
                self.buffer.push(' ');
            }
            _ => {
                // Try to convert key to character
                if let Some(ch) = key_to_char(key) {
                    self.add_char(ch);
                }
            }
        }
        self.last_keypress = Instant::now();
    }

    fn get_last_word(&self) -> Option<String> {
        self.buffer
            .split_whitespace()
            .last()
            .map(|s| s.to_string())
    }

    fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl InputHook {
    /// Create new input hook
    pub fn new() -> Result<Self> {
        Ok(Self {
            devices: Vec::new(),
            word_buffer: Arc::new(RwLock::new(WordBuffer::new())),
            callback: None,
        })
    }

    /// Initialize hook and find keyboard devices
    pub async fn init(&mut self) -> Result<()> {
        let devices = Self::find_keyboard_devices()?;
        self.devices = devices;
        Ok(())
    }

    /// Find all keyboard devices
    fn find_keyboard_devices() -> Result<Vec<Device>> {
        let mut keyboards = Vec::new();

        // Scan /dev/input/event* devices
        for entry in std::fs::read_dir("/dev/input")? {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name() {
                if filename.to_string_lossy().starts_with("event") {
                    if let Ok(device) = Device::open(&path) {
                        // Check if it's a keyboard
                        if device.supported_events().contains(EventType::KEY) {
                            keyboards.push(device);
                        }
                    }
                }
            }
        }

        Ok(keyboards)
    }

    /// Set correction callback
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(String) -> Option<String> + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(callback));
    }

    /// Start listening for keyboard events
    pub async fn start(self) -> Result<()> {
        let mut handles = vec![];

        for mut device in self.devices {
            let word_buffer = Arc::clone(&self.word_buffer);
            let callback = self.callback.clone();

            let handle = tokio::spawn(async move {
                loop {
                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                if event.event_type() == EventType::KEY {
                                    let key = Key::new(event.code());
                                    Self::handle_key_event(
                                        key,
                                        event,
                                        &word_buffer,
                                        &callback,
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading events: {}", e);
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all device listeners
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Handle individual key event
    async fn handle_key_event(
        key: Key,
        event: InputEvent,
        word_buffer: &Arc<RwLock<WordBuffer>>,
        callback: &Option<Arc<dyn Fn(String) -> Option<String> + Send + Sync>>,
    ) {
        // Only process key press events (value == 1)
        if event.value() != 1 {
            return;
        }

        let mut buffer = word_buffer.write().await;
        buffer.add_key(key);

        // Check for word boundaries (space, enter, etc.)
        if matches!(key, Key::KEY_SPACE | Key::KEY_ENTER) {
            if let Some(word) = buffer.get_last_word() {
                if let Some(ref cb) = callback {
                    if let Some(correction) = cb(word.clone()) {
                        // Correction needed
                        println!("Correcting: {} -> {}", word, correction);
                        // TODO: Implement text replacement via uinput
                        Self::replace_text(&word, &correction).await;
                    }
                }
            }
        }
    }

    /// Replace text in the active window
    async fn replace_text(old_text: &str, new_text: &str) {
        log::info!("Replacing '{}' with '{}'", old_text, new_text);
        
        // Get or create virtual keyboard
        let kb_lock = Arc::clone(&VIRTUAL_KEYBOARD);
        let mut kb_guard = kb_lock.lock().await;
        
        if kb_guard.is_none() {
            match VirtualKeyboard::new() {
                Ok(kb) => {
                    log::info!("Virtual keyboard created successfully");
                    *kb_guard = Some(kb);
                }
                Err(e) => {
                    log::error!("Failed to create virtual keyboard: {}. Text replacement disabled.", e);
                    return;
                }
            }
        }
        
        if let Some(keyboard) = kb_guard.as_mut() {
            // Delete old text (backspace for each character)
            if let Err(e) = keyboard.press_backspace(old_text.chars().count()) {
                log::error!("Failed to send backspace: {}", e);
                return;
            }
            
            // Type new text
            if let Err(e) = keyboard.type_text(new_text) {
                log::error!("Failed to type new text: {}", e);
                return;
            }
            
            log::info!("Successfully replaced text");
        }
    }

    /// Get statistics
    pub async fn get_buffer_stats(&self) -> String {
        let buffer = self.word_buffer.read().await;
        format!(
            "Buffer: {} chars, Last keypress: {:?} ago",
            buffer.buffer.len(),
            buffer.last_keypress.elapsed()
        )
    }
}

/// Convert evdev Key to character
fn key_to_char(key: Key) -> Option<char> {
    match key {
        Key::KEY_A => Some('a'),
        Key::KEY_B => Some('b'),
        Key::KEY_C => Some('c'),
        Key::KEY_D => Some('d'),
        Key::KEY_E => Some('e'),
        Key::KEY_F => Some('f'),
        Key::KEY_G => Some('g'),
        Key::KEY_H => Some('h'),
        Key::KEY_I => Some('i'),
        Key::KEY_J => Some('j'),
        Key::KEY_K => Some('k'),
        Key::KEY_L => Some('l'),
        Key::KEY_M => Some('m'),
        Key::KEY_N => Some('n'),
        Key::KEY_O => Some('o'),
        Key::KEY_P => Some('p'),
        Key::KEY_Q => Some('q'),
        Key::KEY_R => Some('r'),
        Key::KEY_S => Some('s'),
        Key::KEY_T => Some('t'),
        Key::KEY_U => Some('u'),
        Key::KEY_V => Some('v'),
        Key::KEY_W => Some('w'),
        Key::KEY_X => Some('x'),
        Key::KEY_Y => Some('y'),
        Key::KEY_Z => Some('z'),
        Key::KEY_SPACE => Some(' '),
        Key::KEY_0 => Some('0'),
        Key::KEY_1 => Some('1'),
        Key::KEY_2 => Some('2'),
        Key::KEY_3 => Some('3'),
        Key::KEY_4 => Some('4'),
        Key::KEY_5 => Some('5'),
        Key::KEY_6 => Some('6'),
        Key::KEY_7 => Some('7'),
        Key::KEY_8 => Some('8'),
        Key::KEY_9 => Some('9'),
        _ => None,
    }
}

/// Virtual keyboard for injecting corrected text
pub struct VirtualKeyboard {
    device: uinput::Device,
    key_delay_ms: u64,
}

impl VirtualKeyboard {
    /// Create a new virtual keyboard device
    pub fn new() -> Result<Self> {
        Self::with_delay(2)
    }
    
    /// Create a new virtual keyboard with custom key delay (in milliseconds)
    pub fn with_delay(delay_ms: u64) -> Result<Self> {
        log::info!("Creating virtual keyboard device...");
        
        // Create uinput device with keyboard capabilities
        let device = uinput::default()
            .context("Failed to open uinput device. Make sure /dev/uinput exists and you have proper permissions (add user to 'input' group)")?
            .name("SmartType Virtual Keyboard")
            .context("Failed to set device name")?
            .event(uinput::event::Keyboard::All)
            .context("Failed to configure keyboard events")?
            .create()
            .context("Failed to create uinput device")?;
        
        log::info!("Virtual keyboard device created successfully");
        
        Ok(Self {
            device,
            key_delay_ms: delay_ms,
        })
    }
    
    /// Type text by sending appropriate key events
    pub fn type_text(&mut self, text: &str) -> Result<()> {
        log::debug!("Typing text: '{}'", text);
        
        for ch in text.chars() {
            self.type_char(ch)?;
        }
        
        Ok(())
    }
    
    /// Type a single character
    fn type_char(&mut self, ch: char) -> Result<()> {
        let (key, needs_shift) = match char_to_key(ch) {
            Some(k) => k,
            None => {
                log::warn!("Cannot type unsupported character: '{}'", ch);
                return Ok(());
            }
        };
        
        // Press shift if needed for uppercase or special chars
        if needs_shift {
            self.press_key(keyboard::Key::LeftShift)?;
        }
        
        // Press and release the key
        self.press_key(key)?;
        self.release_key(key)?;
        
        // Release shift if it was pressed
        if needs_shift {
            self.release_key(keyboard::Key::LeftShift)?;
        }
        
        // Small delay between characters for reliability
        thread::sleep(Duration::from_millis(self.key_delay_ms));
        
        Ok(())
    }
    
    /// Send backspace key events
    pub fn press_backspace(&mut self, count: usize) -> Result<()> {
        log::debug!("Pressing backspace {} times", count);
        
        for _ in 0..count {
            self.press_key(keyboard::Key::BackSpace)?;
            self.release_key(keyboard::Key::BackSpace)?;
            thread::sleep(Duration::from_millis(self.key_delay_ms));
        }
        
        Ok(())
    }
    
    /// Press a key
    fn press_key(&mut self, key: keyboard::Key) -> Result<()> {
        self.device.press(&key)
            .context("Failed to press key")?;
        self.device.synchronize()
            .context("Failed to synchronize after key press")?;
        Ok(())
    }
    
    /// Release a key
    fn release_key(&mut self, key: keyboard::Key) -> Result<()> {
        self.device.release(&key)
            .context("Failed to release key")?;
        self.device.synchronize()
            .context("Failed to synchronize after key release")?;
        Ok(())
    }
}

/// Convert a character to a keyboard key and shift requirement
/// Returns (Key, needs_shift)
fn char_to_key(ch: char) -> Option<(keyboard::Key, bool)> {
    match ch {
        // Lowercase letters
        'a' => Some((keyboard::Key::A, false)),
        'b' => Some((keyboard::Key::B, false)),
        'c' => Some((keyboard::Key::C, false)),
        'd' => Some((keyboard::Key::D, false)),
        'e' => Some((keyboard::Key::E, false)),
        'f' => Some((keyboard::Key::F, false)),
        'g' => Some((keyboard::Key::G, false)),
        'h' => Some((keyboard::Key::H, false)),
        'i' => Some((keyboard::Key::I, false)),
        'j' => Some((keyboard::Key::J, false)),
        'k' => Some((keyboard::Key::K, false)),
        'l' => Some((keyboard::Key::L, false)),
        'm' => Some((keyboard::Key::M, false)),
        'n' => Some((keyboard::Key::N, false)),
        'o' => Some((keyboard::Key::O, false)),
        'p' => Some((keyboard::Key::P, false)),
        'q' => Some((keyboard::Key::Q, false)),
        'r' => Some((keyboard::Key::R, false)),
        's' => Some((keyboard::Key::S, false)),
        't' => Some((keyboard::Key::T, false)),
        'u' => Some((keyboard::Key::U, false)),
        'v' => Some((keyboard::Key::V, false)),
        'w' => Some((keyboard::Key::W, false)),
        'x' => Some((keyboard::Key::X, false)),
        'y' => Some((keyboard::Key::Y, false)),
        'z' => Some((keyboard::Key::Z, false)),
        
        // Uppercase letters
        'A' => Some((keyboard::Key::A, true)),
        'B' => Some((keyboard::Key::B, true)),
        'C' => Some((keyboard::Key::C, true)),
        'D' => Some((keyboard::Key::D, true)),
        'E' => Some((keyboard::Key::E, true)),
        'F' => Some((keyboard::Key::F, true)),
        'G' => Some((keyboard::Key::G, true)),
        'H' => Some((keyboard::Key::H, true)),
        'I' => Some((keyboard::Key::I, true)),
        'J' => Some((keyboard::Key::J, true)),
        'K' => Some((keyboard::Key::K, true)),
        'L' => Some((keyboard::Key::L, true)),
        'M' => Some((keyboard::Key::M, true)),
        'N' => Some((keyboard::Key::N, true)),
        'O' => Some((keyboard::Key::O, true)),
        'P' => Some((keyboard::Key::P, true)),
        'Q' => Some((keyboard::Key::Q, true)),
        'R' => Some((keyboard::Key::R, true)),
        'S' => Some((keyboard::Key::S, true)),
        'T' => Some((keyboard::Key::T, true)),
        'U' => Some((keyboard::Key::U, true)),
        'V' => Some((keyboard::Key::V, true)),
        'W' => Some((keyboard::Key::W, true)),
        'X' => Some((keyboard::Key::X, true)),
        'Y' => Some((keyboard::Key::Y, true)),
        'Z' => Some((keyboard::Key::Z, true)),
        
        // Numbers
        '0' => Some((keyboard::Key::_0, false)),
        '1' => Some((keyboard::Key::_1, false)),
        '2' => Some((keyboard::Key::_2, false)),
        '3' => Some((keyboard::Key::_3, false)),
        '4' => Some((keyboard::Key::_4, false)),
        '5' => Some((keyboard::Key::_5, false)),
        '6' => Some((keyboard::Key::_6, false)),
        '7' => Some((keyboard::Key::_7, false)),
        '8' => Some((keyboard::Key::_8, false)),
        '9' => Some((keyboard::Key::_9, false)),
        
        // Special characters (US keyboard layout)
        ' ' => Some((keyboard::Key::Space, false)),
        '!' => Some((keyboard::Key::_1, true)),
        '@' => Some((keyboard::Key::_2, true)),
        '#' => Some((keyboard::Key::_3, true)),
        '$' => Some((keyboard::Key::_4, true)),
        '%' => Some((keyboard::Key::_5, true)),
        '^' => Some((keyboard::Key::_6, true)),
        '&' => Some((keyboard::Key::_7, true)),
        '*' => Some((keyboard::Key::_8, true)),
        '(' => Some((keyboard::Key::_9, true)),
        ')' => Some((keyboard::Key::_0, true)),
        '-' => Some((keyboard::Key::Minus, false)),
        '_' => Some((keyboard::Key::Minus, true)),
        '=' => Some((keyboard::Key::Equal, false)),
        '+' => Some((keyboard::Key::Equal, true)),
        '[' => Some((keyboard::Key::LeftBrace, false)),
        '{' => Some((keyboard::Key::LeftBrace, true)),
        ']' => Some((keyboard::Key::RightBrace, false)),
        '}' => Some((keyboard::Key::RightBrace, true)),
        '\\' => Some((keyboard::Key::BackSlash, false)),
        '|' => Some((keyboard::Key::BackSlash, true)),
        ';' => Some((keyboard::Key::SemiColon, false)),
        ':' => Some((keyboard::Key::SemiColon, true)),
        '\'' => Some((keyboard::Key::Apostrophe, false)),
        '"' => Some((keyboard::Key::Apostrophe, true)),
        ',' => Some((keyboard::Key::Comma, false)),
        '<' => Some((keyboard::Key::Comma, true)),
        '.' => Some((keyboard::Key::Dot, false)),
        '>' => Some((keyboard::Key::Dot, true)),
        '/' => Some((keyboard::Key::Slash, false)),
        '?' => Some((keyboard::Key::Slash, true)),
        '`' => Some((keyboard::Key::Grave, false)),
        '~' => Some((keyboard::Key::Grave, true)),
        
        // Unsupported characters
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_char() {
        assert_eq!(key_to_char(Key::KEY_A), Some('a'));
        assert_eq!(key_to_char(Key::KEY_SPACE), Some(' '));
        assert_eq!(key_to_char(Key::KEY_ESC), None);
    }

    #[tokio::test]
    async fn test_word_buffer() {
        let mut buffer = WordBuffer::new();
        buffer.add_char('h');
        buffer.add_char('e');
        buffer.add_char('l');
        buffer.add_char('l');
        buffer.add_char('o');
        assert_eq!(buffer.buffer, "hello");
    }
    
    #[test]
    fn test_char_to_key_lowercase() {
        let (_key, shift) = char_to_key('a').unwrap();
        assert_eq!(shift, false);
        
        let (_key, shift) = char_to_key('z').unwrap();
        assert_eq!(shift, false);
    }
    
    #[test]
    fn test_char_to_key_uppercase() {
        let (_key, shift) = char_to_key('A').unwrap();
        assert_eq!(shift, true);
        
        let (_key, shift) = char_to_key('Z').unwrap();
        assert_eq!(shift, true);
    }
    
    #[test]
    fn test_char_to_key_numbers() {
        let (_key, shift) = char_to_key('0').unwrap();
        assert_eq!(shift, false);
        
        let (_key, shift) = char_to_key('5').unwrap();
        assert_eq!(shift, false);
    }
    
    #[test]
    fn test_char_to_key_special_chars() {
        let (_key, shift) = char_to_key(' ').unwrap();
        assert_eq!(shift, false);
        
        let (_key, shift) = char_to_key('!').unwrap();
        assert_eq!(shift, true);
        
        let (_key, shift) = char_to_key('.').unwrap();
        assert_eq!(shift, false);
        
        let (_key, shift) = char_to_key('?').unwrap();
        assert_eq!(shift, true);
    }
    
    #[test]
    fn test_char_to_key_unsupported() {
        // Unicode characters beyond basic ASCII
        assert!(char_to_key('é').is_none());
        assert!(char_to_key('中').is_none());
        assert!(char_to_key('🎉').is_none());
    }
}
