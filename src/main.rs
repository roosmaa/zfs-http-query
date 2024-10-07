mod unix_socket;
mod server;

use std::{env, io};
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tokio::signal;

const DEFAULT_SOCKET_PATH: &str = "/var/run/zfs-http-query.sock";
const DEFAULT_BIN_PATH: &str = "/opt/zfs-http-query/bin";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let socket_path = env::var("ZHQ_SOCKET_PATH").unwrap_or(DEFAULT_SOCKET_PATH.to_string());
    let bin_path = env::var("ZHQ_BIN_PATH").unwrap_or(DEFAULT_BIN_PATH.to_string());

    install_scripts(bin_path).await.expect("failed to install shims");

    unix_socket::serve(
        socket_path,
        server::router(),
        shutdown_signal(),
    ).await;
}

async fn install_scripts<P>(
    bin_path: P,
) -> io::Result<()>
where
    P: Into<PathBuf>,
{
    let bin_path = bin_path.into();
    tokio::fs::create_dir_all(&bin_path).await?;

    let zpool_path = bin_path.join("zpool");
    tokio::fs::write(&zpool_path, include_bytes!("shims/zpool.sh")).await?;
    tokio::fs::set_permissions(&zpool_path, Permissions::from_mode(0o755)).await?;

    Ok(())
}

async fn shutdown_signal() {
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
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}