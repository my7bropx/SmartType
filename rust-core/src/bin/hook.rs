use smarttype_core::{AutocorrectEngine, Config, WordCompleter, hook::InputHook};
use anyhow::Result;
use log::{error, info};
use std::sync::{Arc, RwLock};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("SmartType hook starting...");

    let config = Config::load().unwrap_or_default();
    let engine = Arc::new(RwLock::new(AutocorrectEngine::new(config)?));
    info!("Autocorrect engine ready");

    let completer = Arc::new(RwLock::new(WordCompleter::new()));
    info!("Autocomplete engine ready");

    let mut hook = InputHook::new(Arc::clone(&completer))?;

    hook.set_autocorrect(move |word: &str| -> Option<String> {
        engine.read().unwrap().correct_word(word)
    });

    hook.init().await?;
    info!("Keyboard devices initialised");
    info!("SmartType active — suggestions in popup bar. Tab = first, 1-5 = nth, backspace after space to re-edit.");

    if let Err(e) = hook.start().await {
        error!("Hook error: {}", e);
        return Err(e);
    }

    Ok(())
}
