mod api;
mod inx;
mod shutdown;
mod process;

type Error = Box<dyn std::error::Error>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    std::panic::set_hook(Box::new(|p| {
        tracing::error!("{}", p);
    }));

    let (mut shutdown_from_app, shutdown_notifier, shutdown_signal) = shutdown::shutdown_handles();

    let api_handle = tokio::spawn(api::start(shutdown_signal.clone()));
    // No unused `shutdown_signal`s should hang around because we are waiting for them to be dropped.
    // Because of this we move the `shutdown_signal` into the last task.
    let inx_handle = tokio::spawn(inx::start(shutdown_signal));

    // We wait for either the interrupt signal to arrive or for a component of our system to signal a shutdown.
    tokio::select! {
        _ = process::interupt_or_terminate() => {
            tracing::info!("received ctrl-c or terminate");
        },
        _ = shutdown_from_app.recv() => {
            tracing::info!("received shutdown signal from component");
        },
        _ = api_handle => (),
        res = inx_handle => {
            match res.expect("joining the handle should not fail.") {
                Ok(_) => (),
                Err(err) => tracing::error!("INX task failed with error: {err}"),
            }
        },
    }

    // We send the shutdown signal to all tasks that have an instance of the `shutdown_signal`.
    shutdown_notifier.emit().await;

    tracing::info!("shutdown complete");

    Ok(())
}
