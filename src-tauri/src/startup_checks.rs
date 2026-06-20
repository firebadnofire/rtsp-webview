use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::utils::assets::AssetKey;
use tauri::utils::config::FrontendDist;
use tauri::{Assets, Context};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(800);

fn resolve_path(base_dir: &Path, value: &Path) -> PathBuf {
    if value.is_absolute() {
        value.to_path_buf()
    } else {
        base_dir.join(value)
    }
}

fn verify_external_url(url: &tauri::Url) -> Result<(), String> {
    let host = url
        .host_str()
        .ok_or_else(|| format!("frontend URL '{}' is missing a host", url))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| format!("frontend URL '{}' is missing a usable port", url))?;

    let mut resolved = false;
    let mut last_error = String::new();
    let socket_addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve frontend host '{}': {}", host, error))?;

    for address in socket_addresses {
        resolved = true;
        match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            Ok(_) => return Ok(()),
            Err(error) => {
                last_error = error.to_string();
            }
        }
    }

    if !resolved {
        return Err(format!(
            "frontend URL '{}' did not resolve to any network address",
            url
        ));
    }

    Err(format!(
        "failed to reach frontend URL '{}' within {:?}: {}",
        url, CONNECT_TIMEOUT, last_error
    ))
}

fn verify_asset_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!(
            "frontend asset '{}' does not exist",
            path.display()
        ));
    }
    if !path.is_file() {
        return Err(format!(
            "frontend asset '{}' must be a file",
            path.display()
        ));
    }
    let metadata = std::fs::metadata(path).map_err(|error| {
        format!(
            "failed to inspect frontend asset '{}': {}",
            path.display(),
            error
        )
    })?;
    if metadata.len() == 0 {
        return Err(format!("frontend asset '{}' is empty", path.display()));
    }
    Ok(())
}

fn verify_asset_directory(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!(
            "frontend bundle directory '{}' does not exist",
            path.display()
        ));
    }
    if !path.is_dir() {
        return Err(format!(
            "frontend bundle path '{}' must be a directory",
            path.display()
        ));
    }

    let mut entries = std::fs::read_dir(path).map_err(|error| {
        format!(
            "failed to inspect frontend bundle '{}': {}",
            path.display(),
            error
        )
    })?;
    if entries.next().is_none() {
        return Err(format!(
            "frontend bundle directory '{}' is empty",
            path.display()
        ));
    }

    let index_path = path.join("index.html");
    verify_asset_file(&index_path)
}

pub fn preflight_frontend_dist(
    base_dir: &Path,
    frontend_dist: &FrontendDist,
) -> Result<(), String> {
    match frontend_dist {
        FrontendDist::Url(url) => verify_external_url(url),
        FrontendDist::Directory(path) => {
            let resolved = resolve_path(base_dir, path);
            verify_asset_directory(&resolved)
        }
        FrontendDist::Files(files) => {
            if files.is_empty() {
                return Err("frontend files list is empty".to_string());
            }

            let mut has_index = false;
            for file in files {
                let resolved = resolve_path(base_dir, file);
                verify_asset_file(&resolved)?;
                if resolved
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("index.html"))
                {
                    has_index = true;
                }
            }
            if !has_index {
                return Err("frontend files list is missing index.html".to_string());
            }
            Ok(())
        }
        _ => Err("unsupported frontend configuration URL type".to_string()),
    }
}

pub fn preflight_frontend<A: Assets>(context: &Context<A>) -> Result<(), String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    if cfg!(debug_assertions) {
        if let Some(dev_url) = &context.config().build.dev_url {
            verify_external_url(dev_url)?;
        }
        if let Some(frontend_dist) = &context.config().build.frontend_dist {
            preflight_frontend_dist(&manifest_dir, frontend_dist)?;
        }
        return Ok(());
    }

    let index_key = AssetKey::from("index.html");
    if context.assets().get(&index_key).is_none() {
        return Err("embedded frontend asset '/index.html' is missing".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn fails_when_bundle_directory_is_empty() {
        let temp = tempfile::tempdir().expect("tempdir should create");
        let result =
            preflight_frontend_dist(temp.path(), &FrontendDist::Directory("bundle".into()));
        assert!(result.is_err());
    }

    #[test]
    fn fails_when_index_html_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir should create");
        let bundle = temp.path().join("bundle");
        std::fs::create_dir_all(&bundle).expect("bundle directory should create");
        std::fs::write(bundle.join("app.js"), b"console.log('ok');").expect("app.js should write");

        let result =
            preflight_frontend_dist(temp.path(), &FrontendDist::Directory("bundle".into()));
        assert!(result.is_err());
    }

    #[test]
    fn fails_when_external_frontend_is_unreachable() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);

        let url = tauri::Url::parse(&format!("http://127.0.0.1:{port}")).expect("url should parse");
        let result = preflight_frontend_dist(
            tempfile::tempdir().expect("tempdir").path(),
            &FrontendDist::Url(url),
        );
        assert!(result.is_err());
    }
}
