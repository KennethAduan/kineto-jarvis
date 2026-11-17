use crate::{
    application::assistant::{execute_send, locked, AssistantService},
    domain::{models::StreamEvent, ports::EventSink},
    infrastructure::{
        lm_studio::{self as lm, LmStudioGateway},
        sqlite::Store,
    },
};
use std::{sync::Mutex, time::Duration};
struct TestSink<F>(F);
#[tokio::test]
#[ignore = "requires JARVIS_PROFILE_TEST_DB and a running local LM Studio; reads profile only and uses an in-memory test chat"]
async fn local_model_uses_private_profile_and_jarvis_identity() {
    use crate::domain::{
        ports::ConversationRepository,
        profile::{Profile, ProfileRepository},
    };
    let path = std::env::var_os("JARVIS_PROFILE_TEST_DB").expect("explicit local profile database");
    let db = rusqlite::Connection::open_with_flags(
        std::path::Path::new(&path),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let profile = db
        .query_row(
            "SELECT display_name,content,enabled FROM personal_profile WHERE id=1",
            [],
            |row| {
                Ok(Profile {
                    display_name: row.get(0)?,
                    content: row.get(1)?,
                    enabled: row.get(2)?,
                })
            },
        )
        .unwrap();
    let expected = profile.display_name.to_lowercase();
    let mut store = Store::open(std::path::Path::new(":memory:")).unwrap();
    store.save_profile(&profile).unwrap();
    let conversation = store.create().unwrap();
    let previous = store
        .begin(
            &conversation.id,
            "おはようございます",
            "qwen/qwen3-4b-2507",
            "old-language-turn",
        )
        .unwrap();
    store
        .finish(
            &previous,
            "old-language-turn",
            "おはようございます",
            "failed",
            "Synthetic prior speech failure",
        )
        .unwrap();
    let service =
        AssistantService::new(Box::new(store), Box::new(LmStudioGateway::local().unwrap()));
    let reply = execute_send(
        &service,
        conversation.id,
        "What is my full name, and what is your name? Answer in one short sentence.".into(),
        "qwen/qwen3-4b-2507".into(),
        &TestSink(|_| Ok(())),
    )
    .await
    .unwrap();
    assert!(reply.error.is_none());
    assert!(
        !reply
            .message
            .content
            .chars()
            .any(|c| ('\u{3040}'..='\u{30ff}').contains(&c)),
        "English should remain the default despite earlier Japanese context"
    );
    assert!(
        reply.message.content.to_lowercase().contains(&expected),
        "model must use the stored full name"
    );
    assert!(
        reply.message.content.to_lowercase().contains("jarvis"),
        "assistant identity must remain Jarvis"
    );
}
#[test]
fn search_rejects_blank_or_oversized_input_before_querying_storage() {
    let service = state("http://127.0.0.1:1/v1");
    assert!(service.search("  \n ").is_err());
    assert!(service.search(&"a".repeat(201)).is_err());
    assert!(service.search("valid search").unwrap().is_empty());
}
impl<F: Fn(StreamEvent) -> Result<(), String> + Send + Sync> EventSink for TestSink<F> {
    fn send(&self, event: StreamEvent) -> Result<(), String> {
        (self.0)(event)
    }
}
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Arc},
    thread,
};

// A real HTTP/SSE fixture on an ephemeral loopback port. Never requires or replaces LM Studio.
fn request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header = String::from_utf8(bytes).unwrap();
    let len = header
        .lines()
        .find_map(|line| {
            line.to_lowercase()
                .strip_prefix("content-length:")
                .map(|n| n.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; len];
    stream.read_exact(&mut body).unwrap();
    String::from_utf8(body).unwrap()
}
fn fixture(
    body: &'static str,
    redirect: bool,
) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    let thread = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        request(&mut stream);
        let models = r#"{"data":[{"id":"qwen3.5-9b-fixture"}]}"#;
        if redirect {
            write!(stream,"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/private\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            return;
        }
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",models.len(),models).unwrap();
        drop(stream);
        let (mut stream, _) = listener.accept().unwrap();
        let _ = tx.send(request(&mut stream));
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
        for chunk in body.as_bytes().chunks(3) {
            if stream.write_all(chunk).is_err() {
                break;
            }
        }
    });
    (url, rx, thread)
}
fn state(endpoint: &str) -> AssistantService {
    AssistantService::new(
        Box::new(Store::open(std::path::Path::new(":memory:")).unwrap()),
        Box::new(LmStudioGateway::at(endpoint).unwrap()),
    )
}
const SUCCESS:&str="data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"Hello 🦀\"}}]}\n\ndata: [DONE]\n\n";

#[tokio::test]
async fn real_http_stream_saves_history_and_activity() {
    let (url, rx, server) = fixture(SUCCESS, false);
    let state = state(&url);
    state
        .save_profile(&crate::domain::profile::Profile {
            display_name: "Avery Example".into(),
            content: "Full-stack engineer in Example City.\n\nCONTACT\navery@example.invalid"
                .into(),
            enabled: true,
        })
        .unwrap();
    let conversation = locked(&state.store).unwrap().create().unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let saved = events.clone();
    let channel = TestSink(move |event| {
        saved.lock().unwrap().push(event);
        Ok(())
    });
    let result = execute_send(
        &state,
        conversation.id.clone(),
        "Hello".into(),
        "qwen3.5-9b-fixture".into(),
        &channel,
    )
    .await
    .unwrap();
    assert_eq!(result.message.content, "Hello 🦀");
    assert_eq!(result.message.status, "complete");
    assert!(result.error.is_none());
    assert_eq!(events.lock().unwrap().len(), 3);
    let body: serde_json::Value = serde_json::from_str(&rx.recv().unwrap()).unwrap();
    assert_eq!(body["model"], "qwen3.5-9b-fixture");
    assert_eq!(body["stream"], true);
    assert!(body.get("tools").is_none());
    assert_eq!(body["messages"][1]["content"], "Hello");
    let system = body["messages"][0]["content"].as_str().unwrap();
    assert!(system.contains("Your name is JARVIS"));
    assert!(system.contains("Reply in English by default"));
    assert!(system.contains("Avery Example"));
    assert!(system.contains("Full-stack engineer"));
    assert!(
        !system.contains("avery@example.invalid"),
        "unrequested contact section is not retrieved"
    );
    assert_eq!(
        locked(&state.store)
            .unwrap()
            .messages(&conversation.id)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        locked(&state.store).unwrap().activity().unwrap()[0].status,
        "complete"
    );
    assert!(locked(&state.active).unwrap().is_none());
    server.join().unwrap();
}

#[tokio::test]
async fn truncated_stream_retains_partial_answer_and_reports_failure() {
    let (url, _, server) = fixture(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Partial answer\"}}]}\n\n",
        false,
    );
    let state = state(&url);
    let c = locked(&state.store).unwrap().create().unwrap();
    let result = execute_send(
        &state,
        c.id,
        "Hello".into(),
        "qwen3.5-9b-fixture".into(),
        &TestSink(|_| Ok(())),
    )
    .await
    .unwrap();
    assert_eq!(result.message.status, "failed");
    assert_eq!(result.message.content, "Partial answer");
    assert!(result.error.unwrap().contains("before completion"));
    server.join().unwrap();
}

#[tokio::test]
async fn cancellation_before_inference_saves_status_and_releases_busy_guard() {
    let state = Arc::new(state("http://127.0.0.1:1/v1"));
    let c = locked(&state.store).unwrap().create().unwrap();
    let active = state.clone();
    let channel = TestSink(move |_| {
        // Cancel at Started, before any inference network request.
        if let Some(task) = active.active.lock().unwrap().as_ref() {
            task.cancel.cancel();
        }
        Ok(())
    });
    let result = execute_send(
        &state,
        c.id,
        "Hello".into(),
        "qwen3.5-9b-fixture".into(),
        &channel,
    )
    .await
    .unwrap();
    assert_eq!(result.message.status, "cancelled");
    assert!(result.error.is_none());
    assert!(locked(&state.active).unwrap().is_none());
}

#[tokio::test]
async fn refuses_http_redirect_instead_of_following_it() {
    let (url, _, server) = fixture("", true);
    let error = lm::models_at(&lm::client().unwrap(), &url)
        .await
        .err()
        .unwrap();
    assert!(error.contains("302"));
    server.join().unwrap();
}
