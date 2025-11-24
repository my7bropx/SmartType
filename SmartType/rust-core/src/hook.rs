use anyhow::Result;
use evdev::{Device, EventType, InputEvent, Key};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

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
        // This would use uinput to:
        // 1. Send backspace keys to delete old text
        // 2. Type new text

        // For now, just log the action
        log::info!("Would replace '{}' with '{}'", old_text, new_text);

        // Implementation would look like:
        // - Create virtual keyboard with uinput
        // - Send KEY_BACKSPACE events (old_text.len() times)
        // - Send character events for new_text
        // This requires more complex setup with uinput
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
    // This would use uinput to create a virtual keyboard device
    // Implementation requires uinput crate
}

impl VirtualKeyboard {
    pub fn new() -> Result<Self> {
        // Create virtual keyboard device
        Ok(Self {})
    }

    pub fn type_text(&mut self, _text: &str) -> Result<()> {
        // Send key events to type text
        Ok(())
    }

    pub fn press_backspace(&mut self, _count: usize) -> Result<()> {
        // Send backspace events
        Ok(())
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
}
