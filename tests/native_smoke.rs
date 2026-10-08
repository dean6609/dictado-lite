//! Optional local/CI smoke. Input is synthetic silence, not a user's voice.
use dictado_lite::engine::{verify_model, Backend, Cancellation, Engine, Error};
use std::path::PathBuf;
use std::time::Duration;

#[test]
#[ignore = "requires DICTADO_TEST_MODEL pointing to the hash-verified pinned weights"]
fn model_run_cancel_and_reload() {
    let path =
        PathBuf::from(std::env::var_os("DICTADO_TEST_MODEL").expect("Set DICTADO_TEST_MODEL"));
    verify_model(&path).expect("pinned model integrity");
    let backend = if std::env::var_os("DICTADO_TEST_CPU").is_some() {
        Backend::Cpu
    } else {
        Backend::PreferVulkan
    };
    let mut engine = Engine::load(&path, backend).expect("model and session load");
    let cancel = Cancellation::default();
    let trigger = cancel.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        trigger.cancel();
    });
    let result = engine.run(&vec![0.0; 16_000 * 5], &cancel);
    thread.join().unwrap();
    assert!(
        matches!(result, Err(Error::Cancelled)),
        "partial text must not escape cancellation"
    );
    let result = engine.run(&vec![0.0; 16_000], &Cancellation::default());
    assert!(
        result.is_ok(),
        "a cancelled session must recover: {result:?}"
    );
    drop(engine);
    let mut engine = Engine::load(&path, backend).expect("reload after session/model teardown");
    assert!(engine
        .run(&vec![0.0; 16_000], &Cancellation::default())
        .is_ok());
}
