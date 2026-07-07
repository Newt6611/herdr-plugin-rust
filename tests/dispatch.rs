use std::sync::Arc;

use herdr_plugin::dispatcher::EventDispatcher;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
struct TabCreated {
    id: u64,
}

#[derive(Clone, Debug)]
struct PaneFocused {
    id: u64,
}

async fn record_tab(ctx: Arc<Mutex<Vec<String>>>, event: TabCreated) {
    ctx.lock().await.push(format!("tab:{}", event.id));
}

async fn record_tab_again(ctx: Arc<Mutex<Vec<String>>>, event: TabCreated) {
    ctx.lock().await.push(format!("tab-again:{}", event.id));
}

async fn record_pane(ctx: Arc<Mutex<Vec<String>>>, event: PaneFocused) {
    ctx.lock().await.push(format!("pane:{}", event.id));
}

#[tokio::test]
async fn dispatches_handlers_for_the_concrete_event_type_in_registration_order() {
    let events = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut dispatcher = EventDispatcher::new();

    dispatcher.on::<TabCreated>(record_tab);
    dispatcher.on::<PaneFocused>(record_pane);
    dispatcher.on::<TabCreated>(record_tab_again);

    dispatcher
        .dispatch(events.clone(), TabCreated { id: 1 })
        .await;
    dispatcher
        .dispatch(events.clone(), PaneFocused { id: 9 })
        .await;

    let events = events.lock().await.clone();
    assert_eq!(events, ["tab:1", "tab-again:1", "pane:9"]);
}
