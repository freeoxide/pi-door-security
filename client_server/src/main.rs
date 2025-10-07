//! Pi Door Security Client Agent
//! Main entry point

use pi_door_client::{
    config,
    events::EventBus,
    gpio::{DefaultGpio, GpioController},
    observability,
    state::{new_app_state, StateMachine},
    api,
};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    observability::init_logging()?;
    info!("Pi Door Security Client Agent v{}", pi_door_client::VERSION);

    // Load configuration
    let config = config::load_config()?;
    info!(client_id = %config.system.client_id, "Configuration loaded");

    // Initialize shared state
    let app_state = new_app_state();

    // Initialize event bus
    let (event_bus, mut event_rx) = EventBus::new();

    // Initialize GPIO
    let mut gpio = DefaultGpio::new();
    gpio.initialize().await?;
    info!("GPIO initialized");

    // Set up panic hook for emergency shutdown
    let gpio_clone = gpio.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        error!("PANIC: {:?}", panic_info);
        gpio_clone.emergency_shutdown();
    }));

    let gpio_arc: Arc<dyn GpioController> = Arc::new(gpio);

    // Initialize state machine
    let mut state_machine = StateMachine::new(
        app_state.clone(),
        event_bus.clone(),
        config.timers.clone(),
        config.system.client_id.clone(),
    );

    // Spawn state machine event processing task
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            if let Err(e) = state_machine.process_event(event).await {
                error!(error = %e, "Failed to process event");
            }
        }
        info!("State machine event loop terminated");
    });

    // Create HTTP API router
    let app = api::create_router(app_state.clone(), event_bus.clone());

    // Start HTTP server
    let listener = tokio::net::TcpListener::bind(&config.http.listen_addr)
        .await?;
    info!(addr = %config.http.listen_addr, "HTTP server listening");

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(gpio_arc))
        .await?;

    info!("Server shut down gracefully");
    Ok(())
}

/// Wait for shutdown signal
async fn shutdown_signal(gpio: Arc<dyn GpioController>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }

    // Emergency shutdown GPIO
    info!("Setting GPIO to safe state");
    gpio.emergency_shutdown();
}
