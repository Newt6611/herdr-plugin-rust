use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use herdr_client::{HerdrClient, HerdrError};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_FAKE_ID: AtomicU64 = AtomicU64::new(0);
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn fake_herdr(script: &str) -> PathBuf {
    let id = NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("herdr-client-test-{}-{id}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();

    let path = dir.join("herdr");
    fs::write(&path, script).unwrap();

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }

    path
}

fn script(body: &str) -> String {
    format!("#!/bin/sh\nset -eu\n{body}\n")
}

#[tokio::test]
async fn list_parses_real_session_list_shape() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "session list --json" ]; then
  printf '%s\n' '{"sessions":[{"default":true,"name":"default","running":false,"session_dir":"/Users/newt/.config/herdr","socket_path":"/Users/newt/.config/herdr/herdr.sock"}]}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let sessions = client.session().list().await.unwrap();

    assert_eq!(sessions.sessions.len(), 1);
    let session = &sessions.sessions[0];
    assert!(session.is_default);
    assert_eq!(session.name, "default");
    assert!(!session.running);
    assert_eq!(session.session_dir, Path::new("/Users/newt/.config/herdr"));
    assert_eq!(
        session.socket_path,
        Path::new("/Users/newt/.config/herdr/herdr.sock")
    );
}

#[tokio::test]
async fn new_uses_herdr_bin_path_when_set() {
    let _guard = ENV_LOCK.lock().unwrap();
    let previous = std::env::var_os("HERDR_BIN_PATH");
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "session list --json" ]; then
  printf '%s\n' '{"sessions":[]}'
  exit 0
fi
exit 99"#,
    ));
    std::env::set_var("HERDR_BIN_PATH", &herdr);
    let client = HerdrClient::new();

    let sessions = client.session().list().await.unwrap();

    match previous {
        Some(value) => std::env::set_var("HERDR_BIN_PATH", value),
        None => std::env::remove_var("HERDR_BIN_PATH"),
    }
    assert!(sessions.sessions.is_empty());
}

#[tokio::test]
async fn attach_invokes_session_attach_without_json() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "session attach my-session" ]; then
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    client.session().attach("my-session").await.unwrap();
}

#[tokio::test]
async fn stop_parses_success_json() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "session stop my-session --json" ]; then
  printf '%s\n' '{"stopped":true,"session":{"default":false,"name":"my-session","running":false,"session_dir":"/tmp/my-session","socket_path":"/tmp/my-session/herdr.sock"}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let stopped = client.session().stop("my-session").await.unwrap();

    assert!(stopped.stopped);
    assert_eq!(stopped.session.name, "my-session");
}

#[tokio::test]
async fn delete_parses_success_json() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "session delete missing --json" ]; then
  printf '%s\n' '{"deleted":true,"session":{"default":false,"name":"missing","running":false,"session_dir":"/tmp/missing","socket_path":"/tmp/missing/herdr.sock"}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let deleted = client.session().delete("missing").await.unwrap();

    assert!(deleted.deleted);
    assert_eq!(deleted.session.name, "missing");
}

#[tokio::test]
async fn non_zero_json_error_is_typed() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "session stop missing --json" ]; then
  printf '%s\n' '{"error":{"code":"session_stop_failed","message":"session missing is not running"}}'
  exit 1
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let error = client.session().stop("missing").await.unwrap_err();

    match error {
        HerdrError::CommandFailed { error, .. } => {
            assert_eq!(error.code, "session_stop_failed");
            assert_eq!(error.message, "session missing is not running");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn invalid_json_is_typed() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "session list --json" ]; then
  printf '%s\n' 'not-json'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let error = client.session().list().await.unwrap_err();

    assert!(matches!(error, HerdrError::InvalidJson { .. }));
}

#[tokio::test]
async fn missing_binary_is_typed() {
    let client = HerdrClient::with_binary("/definitely/missing/herdr");

    let error = client.session().list().await.unwrap_err();

    assert!(matches!(error, HerdrError::MissingExecutable { .. }));
}
