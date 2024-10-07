use axum::http::StatusCode;
use axum::Router;
use axum::routing::post;
use subprocess::{Exec, Redirection};
use tokio::task::spawn_blocking;

pub fn router() -> Router {
    Router::new()
        .route("/zpool/list", post(zpool_list))
}


async fn zpool_list(
    arguments: String,
) -> (StatusCode, String) {
    let arguments = match shell_words::split(&arguments) {
        Ok(v) => v,
        Err(err) => return (StatusCode::BAD_REQUEST, err.to_string()),
    };

    // Verify arguments before passing them to zpool list to make sure they're safe
    // Ref: https://openzfs.github.io/openzfs-docs/man/master/8/zpool-list.8.html
    let mut i = 0;
    while i < arguments.len() {
        match arguments[i].as_str() {
            "-j" | "--json-int" | "--json-pool-key-guid"
            | "-g" | "-H" | "-L" | "-p" | "-P" | "-v"
            => i += 1,
            "-o" | "-T" => i += 2, // skip validating next argument
            arg => return (StatusCode::BAD_REQUEST, format!("disallowed argument: {}", arg)),
        }
    }

    spawn_blocking(move || -> (StatusCode, String) {
        let cap = Exec::cmd("zpool").arg("list").args(&arguments)
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Merge)
            .capture();
        match cap {
            Ok(cap) => (StatusCode::OK, cap.stdout_str()),
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        }
    }).await.unwrap_or_else(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}
