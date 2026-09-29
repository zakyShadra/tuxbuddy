use crate::app_state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::net::UnixListener;

/// Binds the Unix socket that `tuxbuddy-hook` connects to and accepts
/// connections forever, one `tokio::spawn`ed task per connection (see
/// `rust-async-patterns`: this app must keep serving other connections
/// while one hook invocation is blocked waiting on a user decision).
pub async fn run(app: AppHandle, state: Arc<AppState>) {
    let path = protocol::socket_path();
    if let Some(dir) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("tuxbuddy: failed to create socket dir {dir:?}: {e}");
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
    }
    // A stale socket file from an unclean previous shutdown would make
    // bind() fail with AddrInUse even though nothing is listening.
    let _ = std::fs::remove_file(&path);

    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("tuxbuddy: failed to bind socket at {path:?}: {e}");
            return;
        }
    };
    println!("tuxbuddy: hook socket listening at {path:?}");

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let app = app.clone();
                let state = state.clone();
                tokio::spawn(async move {
                    crate::socket::router::handle_connection(stream, app, state).await;
                });
            }
            Err(e) => {
                eprintln!("tuxbuddy: socket accept error: {e}");
            }
        }
    }
}

/// Called from `main.rs`/`lib.rs` setup — spawns the accept loop as a
/// background task so it doesn't block Tauri's own startup.
pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    let state = app.state::<Arc<AppState>>().inner().clone();
    tauri::async_runtime::spawn(run(app, state));
}
