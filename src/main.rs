use std::net::{Ipv6Addr, SocketAddr};

use bank_of_dad::{db::Db, router};
use log::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_thread_ids(true).init();

    info!("started");

    let db = Db::new();
    let app = router(db);

    axum::Server::bind(&SocketAddr::from((Ipv6Addr::UNSPECIFIED, 3000)))
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
