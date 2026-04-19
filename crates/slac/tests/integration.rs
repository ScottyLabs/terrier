use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, request::Parts};
use axum::response::IntoResponse;
use axum::routing::get;
use slac::{All, Any, Authorized, Either, Policy, policy};
use tower::ServiceExt;

#[derive(Clone, Default)]
struct AppState {
    inner: Arc<AppStateInner>,
}

#[derive(Default)]
struct AppStateInner {
    is_admin: bool,
    is_member: bool,
    admin_calls: AtomicUsize,
    member_calls: AtomicUsize,
}

impl AppState {
    fn admin() -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                is_admin: true,
                is_member: true,
                ..Default::default()
            }),
        }
    }

    fn member() -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                is_member: true,
                ..Default::default()
            }),
        }
    }

    fn anon() -> Self {
        Self::default()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct AdminProof {
    label: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
struct MemberProof {
    label: &'static str,
}

struct IsAdmin;
impl Policy<AppState> for IsAdmin {
    type Output = AdminProof;
    type Error = StatusCode;

    fn check(
        _parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
        let inner = state.inner.clone();
        async move {
            inner.admin_calls.fetch_add(1, Ordering::SeqCst);
            if inner.is_admin {
                Ok(AdminProof { label: "admin" })
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        }
    }
}

struct IsMember;
impl Policy<AppState> for IsMember {
    type Output = MemberProof;
    type Error = StatusCode;

    fn check(
        _parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
        let inner = state.inner.clone();
        async move {
            inner.member_calls.fetch_add(1, Ordering::SeqCst);
            if inner.is_member {
                Ok(MemberProof { label: "member" })
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        }
    }
}

policy! {
    pub enum DashboardAccess for AppState {
        Admin  = IsAdmin,
        Member = IsMember,
    }
}

async fn send(router: Router) -> (StatusCode, String) {
    let response = router
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn route<H, T>(state: AppState, handler: H) -> Router
where
    H: axum::handler::Handler<T, AppState>,
    T: 'static,
{
    Router::new().route("/", get(handler)).with_state(state)
}

#[tokio::test]
async fn atomic_pass() {
    async fn handler(
        Authorized { data, .. }: Authorized<IsAdmin, AppState>,
    ) -> Result<String, Infallible> {
        Ok(data.label.to_string())
    }

    let (status, body) = send(route(AppState::admin(), handler)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "admin");
}

#[tokio::test]
async fn atomic_reject() {
    async fn handler(_: Authorized<IsAdmin, AppState>) -> &'static str {
        "ok"
    }

    let (status, _) = send(route(AppState::anon(), handler)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn policy_macro_takes_first_match() {
    async fn handler(
        Authorized { data, .. }: Authorized<DashboardAccess, AppState>,
    ) -> impl IntoResponse {
        match data {
            DashboardAccess::Admin(p) => format!("admin:{}", p.label),
            DashboardAccess::Member(p) => format!("member:{}", p.label),
        }
    }

    let state = AppState::admin();
    let (status, body) = send(route(state.clone(), handler)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "admin:admin");
    assert_eq!(state.inner.admin_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.inner.member_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn policy_macro_falls_through() {
    async fn handler(
        Authorized { data, .. }: Authorized<DashboardAccess, AppState>,
    ) -> impl IntoResponse {
        match data {
            DashboardAccess::Admin(p) => format!("admin:{}", p.label),
            DashboardAccess::Member(p) => format!("member:{}", p.label),
        }
    }

    let state = AppState::member();
    let (status, body) = send(route(state.clone(), handler)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "member:member");
    assert_eq!(state.inner.admin_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.inner.member_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn policy_macro_all_reject() {
    async fn handler(_: Authorized<DashboardAccess, AppState>) -> &'static str {
        "ok"
    }

    let (status, _) = send(route(AppState::anon(), handler)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn any_left() {
    async fn handler(
        Authorized { data, .. }: Authorized<Any<IsAdmin, IsMember>, AppState>,
    ) -> impl IntoResponse {
        match data {
            Either::Left(p) => format!("left:{}", p.label),
            Either::Right(p) => format!("right:{}", p.label),
        }
    }

    let (status, body) = send(route(AppState::admin(), handler)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "left:admin");
}

#[tokio::test]
async fn any_right() {
    async fn handler(
        Authorized { data, .. }: Authorized<Any<IsAdmin, IsMember>, AppState>,
    ) -> impl IntoResponse {
        match data {
            Either::Left(p) => format!("left:{}", p.label),
            Either::Right(p) => format!("right:{}", p.label),
        }
    }

    let (status, body) = send(route(AppState::member(), handler)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "right:member");
}

#[tokio::test]
async fn all_pass() {
    async fn handler(
        Authorized { data, .. }: Authorized<All<IsAdmin, IsMember>, AppState>,
    ) -> impl IntoResponse {
        let (a, m) = data;
        format!("{}+{}", a.label, m.label)
    }

    let (status, body) = send(route(AppState::admin(), handler)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "admin+member");
}

#[tokio::test]
async fn all_short_circuits_on_first_failure() {
    async fn handler(_: Authorized<All<IsAdmin, IsMember>, AppState>) -> &'static str {
        "ok"
    }

    let state = AppState::member();
    let (status, _) = send(route(state.clone(), handler)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(state.inner.admin_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.inner.member_calls.load(Ordering::SeqCst), 0);
}
