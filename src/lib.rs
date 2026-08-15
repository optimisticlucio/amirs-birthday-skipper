use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
pub use player::Player;
pub use state::ServerState;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_cookies::{CookieManagerLayer, Cookies};
use tower_http::services::{ServeDir, ServeFile};

use crate::player::PlayerId;

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

async fn index_page(
    State(server_state): State<Arc<Mutex<ServerState>>>,
    cookie_jar: Cookies,
) -> impl IntoResponse {
    let Some(this_user_id): Option<PlayerId> = cookie_jar
        .get("user_id")
        .and_then(|cookie| cookie.value().parse().ok())
    else {
        return Redirect::to("/login").into_response();
    };

    {
        let server_state = server_state.lock().await;
        if !server_state
            .active_game
            .as_ref()
            .is_some_and(|game| game.players.get_player_by_id(&this_user_id).is_some())
        {
            return Redirect::to("/login").into_response();
        }
    }

    let html = tokio::fs::read_to_string("./site_data/index.html")
        .await
        .unwrap();
    Html(html).into_response()
}
