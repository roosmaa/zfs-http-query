use std::fs::Permissions;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use axum::Router;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server;
use hyper_util::service::TowerToHyperService;
use pin_utils::pin_mut;
use tokio::sync::watch;
use tower_service::Service;
use tracing::{error, trace};
use std::os::unix::fs::PermissionsExt;
use futures_util::FutureExt;

pub async fn serve<P, F>(
    socket_path: P,
    app: Router,
    shutdown_signal: F,
)
where
    P: Into<PathBuf>,
    F: Future<Output=()> + Send + 'static,
{
    let socket_path = socket_path.into();
    let _ = tokio::fs::remove_file(&socket_path).await;
    tokio::fs::create_dir_all(socket_path.parent().unwrap()).await.unwrap();

    let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();

    // Make the socket accessible to everyone
    tokio::fs::set_permissions(&socket_path, Permissions::from_mode(0o777)).await.unwrap();

    let mut make_service = app.into_make_service();

    let (signal_tx, signal_rx) = watch::channel(());
    let signal_tx = Arc::new(signal_tx);
    tokio::spawn(async move {
        shutdown_signal.await;
        trace!("received graceful shutdown signal. Telling tasks to shutdown");
        drop(signal_rx);
    });

    let (close_tx, close_rx) = watch::channel(());

    loop {
        let (unix_stream, remote_addr) = tokio::select! {
            conn = listener.accept() => {
                match conn {
                    Ok(conn) => conn,
                    Err(err) => {
                        error!("failed to accept connection: {}", err);
                        continue;
                    }
                }
            }
            _ = signal_tx.closed() => {
                trace!("shutdown signal received, not accepting new connections");
                break;
            }
        };

        trace!("connection {remote_addr:?} accepted");

        let tower_service = make_service
            .call(&unix_stream)
            .await
            .unwrap_or_else(|never| match never {});

        let unix_stream = TokioIo::new(unix_stream);

        let hyper_service = TowerToHyperService::new(tower_service);

        let signal_tx = Arc::clone(&signal_tx);

        let close_rx = close_rx.clone();

        tokio::spawn(async move {
            let builder = server::conn::auto::Builder::new(TokioExecutor::new());
            let conn = builder.serve_connection_with_upgrades(unix_stream, hyper_service);
            pin_mut!(conn);

            let signal_closed = signal_tx.closed().fuse();
            pin_mut!(signal_closed);

            loop {
                tokio::select! {
                    result = conn.as_mut() => {
                        if let Err(err) = result {
                            trace!("failed to serve connection: {err:#}");
                        }
                        break;
                    }
                    _ = &mut signal_closed => {
                        trace!("signal received in task, starting graceful shutdown");
                        conn.as_mut().graceful_shutdown();
                    }
                }
            }

            trace!("connection {remote_addr:?} closed");

            drop(close_rx);
        });
    }

    drop(close_rx);
    drop(listener);

    trace!(
        "waiting for {} task(s) to finish",
        close_tx.receiver_count()
    );
    close_tx.closed().await;
}
