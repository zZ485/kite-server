//! The services module is which accepts and processes requests for client and
//! then calls business logic functions. Server controls database as it do
//! some permission check in acl_middleware

use std::io::Read;

use actix_web::http::HeaderValue;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Executor;
use wechat_sdk::client::{WeChatClient, WeChatClientBuilder};

use crate::bridge::AgentManager;
use crate::config::CONFIG;

mod auth;
mod handlers;
mod middlewares;
mod response;

#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) agents: AgentManager,
    wx_client: WeChatClient,
}

pub async fn server_main() -> std::io::Result<()> {
    // Create database pool.
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .after_connect(|conn| {
            Box::pin(async move {
                conn.execute("SET TIME ZONE 'Asia/Shanghai';").await?;
                Ok(())
            })
        })
        .connect(&CONFIG.server.db)
        .await
        .expect("Could not create database pool");

    // Logger
    set_logger("kite.log");
    let log_string = "%a - - [%t] \"%r\" %s %b %D \"%{User-Agent}i\"";

    // Load white list
    let mut file = std::fs::File::open("ip-whitelist.txt")
        .expect("Failed to open ip-whitelist.txt, you should copy one from ./deploy/");
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();
    drop(file);

    // Wechat server side API client
    let wx_client = WeChatClientBuilder::new()
        .appid(&CONFIG.wechat.appid)
        .secret(&CONFIG.wechat.secret)
        .build();

    let agents = AgentManager::new(&CONFIG.host.bind);
    let _agents = agents.clone();
    tokio::spawn(async move {
        _agents.listen().await;
    });

    let app_state = AppState {
        pool: pool.clone(),
        agents: agents.clone(),
        wx_client,
    };

    use crate::models::sc::activity_update_daemon;

    tokio::spawn(activity_update_daemon(pool, agents));

    // Run actix-web services.
    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(middlewares::Auth {})
            .wrap(middlewares::Reject::new(&buffer))
            .wrap(actix_web::middleware::Compress::default())
            .wrap(actix_web::middleware::Logger::new(log_string))
            .app_data(web::Data::new(app_state.clone()))
            .configure(routes)
    });

    // Unix socket address.
    if CONFIG.server.bind.starts_with('/') {
        #[cfg(unix)]
        {
            server = server.bind_uds(&CONFIG.server.bind)?;
        }

        #[cfg(not(unix))]
        {
            panic!("Could not bind unix socket on a not-unix machine.");
        }
    } else {
        server = server.bind(&CONFIG.server.bind.as_str())?;
    }

    server.run().await
}

fn routes(app: &mut web::ServiceConfig) {
    use handlers::*;

    app.service(
        // API scope: version 1
        web::scope("/api/v1")
            // API index greet :D
            .route("/", web::get().to(|| HttpResponse::Ok().body("Hello world")))
            // User routes
            .service(user::login)
            .service(user::bind_authentication)
            .service(user::list_users)
            .service(user::create_user)
            .service(user::get_user_detail)
            .service(user::update_user_detail)
            .service(user::get_user_identity)
            .service(user::set_user_identity)
            // Freshman routes
            .service(freshman::get_basic_info)
            .service(freshman::update_account)
            .service(freshman::get_roommate)
            .service(freshman::get_classmate)
            .service(freshman::get_people_familiar)
            .service(freshman::get_analysis_data)
            .service(freshman::post_analysis_log)
            // Attachment routes
            .service(attachment::query_attachment)
            .service(attachment::upload_file)
            .service(attachment::list_attachments)
            // Motto routes
            .service(motto::get_one_motto)
            // Event and activity routes
            .service(event::list_events)
            .service(event::get_sc_score_list)
            .service(event::get_sc_score)
            .service(event::get_sc_event_list)
            .service(event::get_sc_event_detail)
            .service(event::apply_sc_event_activity)
            // Edu management and course-related routes
            .service(edu::query_available_classrooms)
            .service(edu::query_timetable)
            .service(edu::query_score)
            .service(edu::get_school_start_date)
            .service(edu::get_school_schedule)
            .service(edu::get_timetable_export_url)
            .service(edu::export_timetable_as_calendar)
            .service(edu::query_score_detail)
            .service(edu::get_exam_arrangement)
            // System status routes
            .service(status::get_timestamp)
            .service(status::ping_agent)
            .service(status::get_agent_list)
            // Pay and room balance
            .service(pay::query_room_balance)
            .service(pay::query_room_bills_by_day)
            .service(pay::query_room_bills_by_hour)
            .service(pay::query_room_consumption_rank)
            // Get Notices
            .service(notice::get_notices)
            // Search module
            .service(search::search)
            // Mall module
            .service(mall::query_textbook)
            .service(mall::get_goods_sorts)
            .service(mall::get_goods_list)
            .service(mall::get_goods_list_by_sort)
            .service(mall::get_goods_list_by_keyword)
            .service(mall::get_goods_by_id)
            .service(mall::publish_goods)
            .service(mall::update_goods)
            .service(mall::delete_goods)
            .service(mall::publish_comment)
            .service(mall::delete_comment)
            .service(mall::get_comments)
            .service(mall::update_num_like)
            .service(mall::append_wish)
            .service(mall::cancel_wish)
            .service(mall::get_wishes)
            // Address book
            .service(contact::query_all_telephone)
            // Library
            .service(library::query_books)
            .service(library::query_book_holding)
            .service(library::query_book_detail)
            // Expense
            .service(pay::query_expense)
            .service(pay::fetch_expense),
    );
}

fn set_logger(path: &str) {
    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, _| out.finish(format_args!("{}", message)))
        .level(log::LevelFilter::Off)
        .level_for("actix_web", log::LevelFilter::Info)
        .chain(fern::log_file(path).expect("Could not open log file."))
        .apply()
        .expect("Failed to set logger.");
}

/// User Jwt token carried in each request.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct JwtToken {
    /// UID of current user.
    pub uid: i32,
    /// current user role.
    pub is_admin: bool,
}

fn get_auth_bearer_value(auth_string: &HeaderValue) -> Option<&str> {
    // https://docs.rs/actix-web/2.0.0/actix_web/http/header/struct.HeaderValue.html#method.to_str
    // Note: to_str().unwrap() will panic when value string contains non-visible chars.
    if let Ok(auth_string) = auth_string.to_str() {
        // Authorization: <Type> <Credentials>
        if let Some(token) = auth_string.strip_prefix("Bearer ") {
            return Some(token);
        }
    }
    None
}
