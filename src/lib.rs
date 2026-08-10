use axum::{
    Router,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
pub use player::Player;
pub use state::ServerState;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_cookies::{CookieManagerLayer, Cookies};
use tower_http::services::{ServeDir, ServeFile};

pub mod login;
pub mod player;
pub mod state;
pub mod utils;
pub mod websocket;

pub fn router(state: ServerState) -> Router<()> {
    Router::new()
        .route("/", get(index_page))
        .route("/websocket", get(websocket::websocket_handler))
        .route(
            "/login",
            post(login::login_logic_handler).get_service(ServeFile::new("./site_data/login.html")),
        )
        .fallback_service(axum::routing::get_service(ServeDir::new("./site_data")))
        .layer(CookieManagerLayer::new())
        .with_state(Arc::new(Mutex::new(state)))
}

async fn index_page(cookie_jar: Cookies) -> impl IntoResponse {
    if cookie_jar.get("user_id").is_none() {
        return Redirect::to("/login").into_response();
    }
    let html = tokio::fs::read_to_string("./site_data/index.html")
        .await
        .unwrap();
    Html(html).into_response()
}
