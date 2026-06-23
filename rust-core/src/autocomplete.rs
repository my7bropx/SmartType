use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct WordCompleter {
    /// All words with their frequency scores
    words: BTreeMap<String, u32>,
    /// User-specific learned boosts (persisted separately)
    learned: HashMap<String, u32>,
    /// Where to persist learned words (None = in-memory only)
    pub learned_path: Option<PathBuf>,
}

// ── Frequency-weighted common English words ───────────────────────────────────

const COMMON_WORDS: &[(&str, u32)] = &[
    // Top function words
    ("about", 900), ("above", 800), ("after", 850), ("again", 820), ("against", 780),
    ("already", 790), ("also", 870), ("although", 750), ("always", 800), ("among", 720),
    ("another", 830), ("answer", 750), ("anything", 780), ("around", 800), ("away", 820),
    // B
    ("back", 880), ("because", 860), ("become", 820), ("before", 870), ("begin", 790),
    ("being", 830), ("below", 760), ("better", 850), ("between", 840), ("big", 830),
    ("both", 820), ("bring", 780), ("build", 800), ("business", 820), ("but", 950),
    // C
    ("call", 840), ("came", 800), ("care", 810), ("carry", 770), ("change", 840),
    ("check", 820), ("child", 810), ("children", 800), ("city", 800), ("clear", 810),
    ("close", 820), ("come", 880), ("coming", 800), ("continue", 790), ("country", 820),
    ("could", 890), ("create", 820),
    // D
    ("data", 830), ("day", 870), ("decide", 780), ("different", 830), ("does", 880),
    ("doing", 820), ("done", 840), ("down", 870), ("during", 800),
    // E
    ("each", 840), ("early", 800), ("easy", 820), ("either", 780), ("else", 840),
    ("enough", 810), ("even", 870), ("every", 850), ("example", 820), ("except", 780),
    // F
    ("fact", 800), ("fall", 800), ("family", 820), ("far", 820), ("fast", 810),
    ("feel", 830), ("few", 830), ("find", 860), ("follow", 800), ("force", 790),
    ("form", 820), ("found", 840), ("free", 820), ("friend", 820), ("from", 920),
    ("full", 820), ("future", 810),
    // G
    ("general", 790), ("get", 920), ("give", 860), ("given", 800), ("global", 780),
    ("good", 890), ("great", 850), ("group", 820), ("grow", 790),
    // H
    ("hand", 820), ("happen", 810), ("hard", 820), ("have", 950), ("head", 820),
    ("hear", 800), ("help", 870), ("here", 890), ("high", 830), ("his", 930),
    ("hold", 800), ("home", 840), ("hope", 810), ("however", 810), ("human", 810),
    // I
    ("idea", 830), ("important", 820), ("include", 800), ("information", 810),
    ("inside", 790), ("instead", 800), ("interest", 810), ("into", 890), ("issue", 810),
    // J-K
    ("just", 900), ("keep", 840), ("kind", 810), ("know", 890), ("knowledge", 800),
    // L
    ("large", 800), ("last", 870), ("later", 820), ("learn", 830), ("leave", 820),
    ("less", 820), ("level", 820), ("life", 860), ("light", 810), ("like", 900),
    ("line", 830), ("list", 820), ("little", 840), ("live", 840), ("local", 800),
    ("long", 860), ("look", 870), ("love", 840),
    // M
    ("main", 810), ("make", 900), ("many", 900), ("matter", 810), ("mean", 840),
    ("message", 820), ("might", 840), ("mind", 830), ("model", 800), ("money", 820),
    ("month", 810), ("more", 920), ("most", 890), ("move", 820), ("much", 890),
    ("must", 870), ("myself", 810),
    // N
    ("name", 860), ("need", 880), ("never", 850), ("next", 860), ("night", 820),
    ("nothing", 820), ("number", 830), ("now", 900),
    // O
    ("often", 820), ("only", 900), ("open", 830), ("order", 820), ("other", 900),
    ("our", 920), ("over", 880), ("own", 870),
    // P
    ("part", 840), ("pass", 800), ("people", 890), ("person", 830), ("place", 840),
    ("plan", 810), ("play", 810), ("point", 830), ("possible", 820), ("power", 810),
    ("problem", 830), ("process", 820), ("public", 800), ("put", 870),
    // Q
    ("question", 820), ("quick", 810), ("quite", 820),
    // R
    ("read", 840), ("real", 850), ("reason", 830), ("receive", 800), ("recent", 800),
    ("remove", 800), ("replace", 800), ("result", 830), ("return", 840), ("right", 880),
    ("run", 860),
    // S
    ("same", 870), ("save", 810), ("search", 820), ("second", 820), ("seem", 820),
    ("send", 820), ("service", 820), ("should", 890), ("show", 860), ("simple", 810),
    ("since", 840), ("small", 840), ("some", 910), ("something", 860), ("sometimes", 820),
    ("start", 850), ("state", 830), ("still", 870), ("stop", 840), ("such", 900),
    ("sure", 840), ("system", 840),
    // T
    ("take", 890), ("than", 910), ("that", 950), ("their", 920), ("them", 910),
    ("then", 910), ("there", 920), ("these", 900), ("they", 940), ("thing", 870),
    ("think", 870), ("this", 970), ("though", 840), ("through", 860), ("time", 920),
    ("together", 820), ("too", 900), ("toward", 800), ("true", 830), ("try", 860),
    ("turn", 820), ("type", 820),
    // U
    ("under", 820), ("until", 820), ("upon", 800), ("use", 900), ("used", 880),
    ("using", 860), ("user", 840),
    // V-W
    ("value", 820), ("very", 920), ("view", 810), ("want", 880), ("watch", 810),
    ("water", 820), ("well", 900), ("what", 950), ("when", 950), ("where", 910),
    ("whether", 820), ("which", 930), ("while", 860), ("who", 940), ("will", 940),
    ("with", 960), ("without", 850), ("word", 820), ("work", 880), ("world", 850),
    ("would", 930), ("write", 830),
    // Y
    ("year", 850), ("yet", 830), ("your", 940), ("yourself", 810),
    // Tech terms
    ("algorithm", 750), ("application", 800), ("array", 780), ("async", 770),
    ("boolean", 760), ("branch", 760), ("buffer", 770), ("cache", 770),
    ("callback", 760), ("channel", 760), ("class", 800), ("clone", 760),
    ("closure", 750), ("code", 840), ("compile", 770), ("config", 800),
    ("connect", 790), ("const", 780), ("container", 770), ("context", 790),
    ("database", 790), ("debug", 790), ("default", 820), ("deploy", 770),
    ("device", 800), ("directory", 780), ("docker", 760), ("download", 790),
    ("dynamic", 760), ("encrypt", 750), ("endpoint", 760), ("enum", 770),
    ("environment", 790), ("error", 840), ("event", 800), ("exception", 760),
    ("execute", 770), ("export", 780), ("extension", 780), ("format", 800),
    ("framework", 770), ("frontend", 760), ("function", 820), ("garbage", 740),
    ("generic", 760), ("global", 780), ("handler", 760), ("hash", 770),
    ("header", 780), ("hostname", 750), ("implement", 780), ("import", 790),
    ("index", 800), ("initialize", 760), ("input", 820), ("install", 790),
    ("interface", 790), ("iterator", 760), ("library", 800), ("linux", 790),
    ("loop", 820), ("memory", 800), ("method", 810), ("module", 800),
    ("mutex", 740), ("network", 800), ("null", 790), ("object", 820),
    ("option", 820), ("output", 820), ("package", 800), ("parameter", 780),
    ("parser", 760), ("pointer", 770), ("python", 780), ("queue", 760),
    ("reference", 790), ("repository", 770), ("request", 800), ("response", 800),
    ("runtime", 770), ("script", 790), ("server", 820), ("socket", 790),
    ("stack", 790), ("string", 820), ("struct", 790), ("thread", 790),
    ("token", 780), ("variable", 800), ("vector", 780), ("version", 810),
    ("virtual", 790), ("window", 810), ("wrapper", 760), ("yield", 760),
];

// ── WordCompleter ──────────────────────────────────────────────────────────────

impl WordCompleter {
    pub fn new() -> Self {
        let learned_path = Self::default_learned_path();
        let learned: HashMap<String, u32> = learned_path
            .as_deref()
            .and_then(|p| Self::load_learned(p).ok())
            .unwrap_or_default();

        let mut words: BTreeMap<String, u32> = BTreeMap::new();

        for path in &["/usr/share/dict/words", "/usr/share/dict/american-english"] {
            if let Ok(content) = std::fs::read_to_string(path) {
                for word in content.lines() {
                    let w = word.to_lowercase();
                    if w.len() >= 2 && w.len() <= 20 && w.chars().all(|c| c.is_ascii_alphabetic()) {
                        words.entry(w).or_insert(100);
                    }
                }
                log::info!("Loaded system wordlist from {}", path);
                break;
            }
        }

        for &(word, freq) in COMMON_WORDS {
            let entry = words.entry(word.to_string()).or_insert(0);
            *entry = (*entry).max(freq);
        }

        for (word, &boost) in &learned {
            let entry = words.entry(word.clone()).or_insert(0);
            *entry = entry.saturating_add(boost);
        }

        log::info!("WordCompleter: {} words ready", words.len());
        Self { words, learned, learned_path }
    }

    // ── Lookup ────────────────────────────────────────────────────────────────

    pub fn contains(&self, word: &str) -> bool {
        self.words.contains_key(&word.to_lowercase())
    }

    // ── Completions (per-keystroke, while typing) ─────────────────────────────

    /// Returns up to `max_count` completions for a prefix being typed.
    /// Prefix matches come first; edit-distance-1 fills remaining slots so
    /// small typos in the prefix (e.g. "hlep" → "help") still get suggestions.
    pub fn suggest(&self, prefix: &str, max_count: usize) -> Vec<String> {
        if prefix.len() < 2 {
            return Vec::new();
        }
        let pl = prefix.to_lowercase();
        let upper = prefix_upper_bound(&pl);

        let mut candidates: Vec<(String, u32, u8)> = self
            .words
            .range(pl.clone()..upper)
            .filter(|(k, _)| k.as_str() != pl.as_str())
            .map(|(k, &v)| (k.clone(), v, 0u8))
            .collect();

        if candidates.len() < max_count && pl.len() >= 3 {
            let mut seen: HashSet<String> =
                candidates.iter().map(|(w, _, _)| w.clone()).collect();
            seen.insert(pl.clone());
            for c in generate_edit1(&pl) {
                if seen.insert(c.clone()) {
                    if let Some(&freq) = self.words.get(&c) {
                        candidates.push((c, freq, 1));
                    }
                }
            }
        }

        sort_candidates(&mut candidates);
        candidates.into_iter().take(max_count).map(|(w, _, _)| w).collect()
    }

    // ── Corrections (one-shot, after space on a completed word) ───────────────

    /// Corrections for a complete word that may be misspelled.
    /// Returns empty if the word is already in the dictionary.
    pub fn correct(&self, word: &str, max_count: usize) -> Vec<String> {
        let wl = word.to_lowercase();
        if wl.len() < 2 || self.words.contains_key(&wl) {
            return Vec::new();
        }

        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(wl.clone());
        let mut candidates: Vec<(String, u32, u8)> = Vec::new();

        for c in generate_edit1(&wl) {
            if seen.insert(c.clone()) {
                if let Some(&freq) = self.words.get(&c) {
                    candidates.push((c, freq, 1));
                }
            }
        }

        // Edit-2 for short words when edit-1 is sparse
        if candidates.len() < max_count && wl.len() <= 6 {
            'outer: for e1 in generate_edit1(&wl) {
                for c in generate_edit1(&e1) {
                    if candidates.len() >= max_count * 4 {
                        break 'outer;
                    }
                    if seen.insert(c.clone()) {
                        if let Some(&freq) = self.words.get(&c) {
                            candidates.push((c, freq, 2));
                        }
                    }
                }
            }
        }

        sort_candidates(&mut candidates);
        candidates.into_iter().take(max_count).map(|(w, _, _)| w).collect()
    }

    // ── Learning ──────────────────────────────────────────────────────────────

    /// Boost a word's score (called on suggestion acceptance or word completion).
    /// Returns a (json, path) snapshot the caller can persist asynchronously,
    /// so this method never does blocking I/O.
    pub fn learn(&mut self, word: &str) -> Option<(String, PathBuf)> {
        let w = word.to_lowercase();
        if w.len() < 2 || !w.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        let freq = self.words.entry(w.clone()).or_insert(100);
        *freq = freq.saturating_add(10);
        *self.learned.entry(w).or_insert(0) += 10;

        let path = self.learned_path.clone()?;
        let json = serde_json::to_string(&self.learned).ok()?;
        Some((json, path))
    }

    // ── Persistence helpers ───────────────────────────────────────────────────

    fn default_learned_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "smarttype", "smarttype")
            .map(|d| d.data_dir().join("learned_words.json"))
    }

    fn load_learned(path: &Path) -> anyhow::Result<HashMap<String, u32>> {
        let s = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&s)?)
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn sort_candidates(candidates: &mut Vec<(String, u32, u8)>) {
    candidates.sort_by(|a, b| {
        a.2.cmp(&b.2) // prefix matches (tier 0) before edit-distance (1, 2)
            .then_with(|| b.1.cmp(&a.1)) // higher frequency first
            .then_with(|| a.0.len().cmp(&b.0.len())) // shorter root word first
            .then_with(|| a.0.cmp(&b.0)) // stable alphabetical fallback
    });
}

/// All strings at edit-distance-1 from `word` (Norvig spell-checker technique).
fn generate_edit1(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    let mut out = Vec::with_capacity(n + n + n * 25 + (n + 1) * 26);

    // Deletions
    for i in 0..n {
        out.push(
            chars[..i]
                .iter()
                .copied()
                .chain(chars[i + 1..].iter().copied())
                .collect(),
        );
    }

    // Transpositions
    for i in 0..n.saturating_sub(1) {
        let mut w = chars.clone();
        w.swap(i, i + 1);
        out.push(w.into_iter().collect());
    }

    // Substitutions
    for i in 0..n {
        for c in 'a'..='z' {
            if c != chars[i] {
                let mut w = chars.clone();
                w[i] = c;
                out.push(w.into_iter().collect());
            }
        }
    }

    // Insertions
    for i in 0..=n {
        for c in 'a'..='z' {
            out.push(
                chars[..i]
                    .iter()
                    .copied()
                    .chain(std::iter::once(c))
                    .chain(chars[i..].iter().copied())
                    .collect(),
            );
        }
    }

    out
}

/// Exclusive upper bound for prefix range queries.
fn prefix_upper_bound(prefix: &str) -> String {
    let mut bytes = prefix.as_bytes().to_vec();
    for i in (0..bytes.len()).rev() {
        if bytes[i] < 0xFF {
            bytes[i] += 1;
            bytes.truncate(i + 1);
            return String::from_utf8(bytes).unwrap_or_else(|_| "\u{10FFFF}".to_string());
        }
    }
    "\u{10FFFF}".to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mini() -> WordCompleter {
        let mut words = BTreeMap::new();
        for &(w, f) in COMMON_WORDS {
            words.insert(w.to_string(), f);
        }
        WordCompleter { words, learned: HashMap::new(), learned_path: None }
    }

    #[test]
    fn prefix_matches_returned_first() {
        let c = mini();
        let s = c.suggest("hel", 5);
        assert!(!s.is_empty());
        assert!(s[0].starts_with("hel"), "first result should be a prefix match: {:?}", s);
    }

    #[test]
    fn typo_in_prefix_still_suggests() {
        let c = mini();
        // "hlep" → "help" is one transposition
        let s = c.suggest("hlep", 5);
        assert!(s.contains(&"help".to_string()), "expected 'help' in {:?}", s);
    }

    #[test]
    fn correct_catches_transposition() {
        let c = mini();
        let s = c.correct("hlep", 5);
        assert!(s.contains(&"help".to_string()), "expected 'help' in {:?}", s);
    }

    #[test]
    fn correct_empty_for_known_word() {
        let c = mini();
        assert!(c.correct("help", 5).is_empty());
    }

    #[test]
    fn contains_basic() {
        let c = mini();
        assert!(c.contains("help"));
        assert!(!c.contains("hlep"));
    }

    #[test]
    fn exact_word_not_in_suggestions() {
        let c = mini();
        assert!(!c.suggest("help", 5).contains(&"help".to_string()));
    }

    #[test]
    fn learn_boosts_score() {
        let mut c = mini();
        let before = *c.words.get("help").unwrap_or(&0);
        c.learn("help");
        assert!(*c.words.get("help").unwrap_or(&0) > before);
    }

    #[test]
    fn prefix_upper_bound_basic() {
        assert_eq!(prefix_upper_bound("hel"), "hem");
        assert_eq!(prefix_upper_bound("ab"), "ac");
    }
}
