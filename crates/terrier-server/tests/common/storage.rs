use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::get,
};

pub const BUCKET: &str = "terrier";

/// An in-memory, path-style S3 endpoint for one bucket.
#[derive(Default)]
pub struct FakeS3 {
    pub objects: Mutex<HashMap<String, (String, Bytes)>>,
    pub fail: AtomicBool,
}

impl FakeS3 {
    pub async fn start() -> (String, Arc<Self>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let store = Arc::new(Self::default());
        let router = Router::new()
            .route("/{bucket}", get(head_bucket))
            .route(
                "/{bucket}/{*key}",
                get(get_object).put(put_object).delete(delete_object),
            )
            .layer(DefaultBodyLimit::disable())
            .with_state(store.clone());
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        (url, store, task)
    }

    fn unavailable(&self) -> Option<Response> {
        self.fail
            .load(Ordering::SeqCst)
            .then(|| StatusCode::SERVICE_UNAVAILABLE.into_response())
    }
}

async fn head_bucket(Path(bucket): Path<String>) -> StatusCode {
    if bucket == BUCKET {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn put_object(
    State(store): State<Arc<FakeS3>>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(response) = store.unavailable() {
        return response;
    }
    assert_eq!(bucket, BUCKET);
    let content_type = headers[CONTENT_TYPE].to_str().unwrap().to_owned();
    store
        .objects
        .lock()
        .unwrap()
        .insert(key, (content_type, body));
    StatusCode::OK.into_response()
}

async fn get_object(
    State(store): State<Arc<FakeS3>>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    if let Some(response) = store.unavailable() {
        return response;
    }
    assert_eq!(bucket, BUCKET);
    match store.objects.lock().unwrap().get(&key) {
        Some((content_type, body)) => {
            ([(CONTENT_TYPE, content_type.clone())], body.clone()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn delete_object(
    State(store): State<Arc<FakeS3>>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    if let Some(response) = store.unavailable() {
        return response;
    }
    assert_eq!(bucket, BUCKET);
    store.objects.lock().unwrap().remove(&key);
    StatusCode::NO_CONTENT.into_response()
}
