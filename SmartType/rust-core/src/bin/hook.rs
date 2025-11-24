use smarttype_core::{SmartType, hook::InputHook};
use anyhow::Result;
use log::{info, error};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("Starting SmartType input hook...");

    // Create SmartType instance
    let smarttype = Arc::new(SmartType::new().await?);
    info!("SmartType engine initialized");

    // Create input hook
    let mut hook = InputHook::new()?;
    info!("Input hook created");

    // Initialize keyboard devices
    hook.init().await?;
    info!("Keyboard devices found and initialized");

    // Set correction callback
    let smarttype_clone = Arc::clone(&smarttype);
    hook.set_callback(move |word| {
        let st = Arc::clone(&smarttype_clone);
        tokio::runtime::Handle::current().block_on(async move {
            st.correct_word(&word).await.unwrap_or(None)
        })
    });

    info!("Starting keyboard event listener...");
    info!("SmartType is now active!");

    // Start listening (this blocks)
    if let Err(e) = hook.start().await {
        error!("Error in input hook: {}", e);
        return Err(e);
    }

    Ok(())
}
