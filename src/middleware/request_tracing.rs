use std::net::SocketAddr;

use axum::{
    extract::ConnectInfo,
    http::{HeaderValue, Request},
    middleware::Next,
    response::IntoResponse,
};
use log::info;

#[derive(Clone)]
pub struct RequestTraceData {
    id: String,
}

impl RequestTraceData {
    pub fn get_id(&self) -> String {
        return self.id.clone();
    }
}

fn get_remote_ip_addr<T>(req: &Request<T>) -> String {
    let connect_info = req.extensions().get::<ConnectInfo<SocketAddr>>().copied();

    match connect_info {
        Some(socket_addr) => socket_addr.ip().to_string(),
        None => String::from("unknown"),
    }
}

fn get_header_or<T>(req: &Request<T>, key: String) -> String {
    match req.headers().get(key) {
        Some(header) => header.to_str().unwrap().to_string(),
        None => String::from("not-set"),
    }
}

pub async fn request_tracing<T>(mut req: Request<T>, next: Next<T>) -> impl IntoResponse {
    let request_id = nanoid::nanoid!(10);

    info!(
        "[{}] {} '{}' {} {}",
        request_id,
        get_remote_ip_addr(&req),
        get_header_or(&req, String::from("user-agent")),
        req.method().as_str(),
        req.uri().to_string(),
    );

    req.extensions_mut().insert(RequestTraceData {
        id: request_id.clone(),
    });
    let mut response = next.run(req).await;

    response
        .headers_mut()
        .insert("X-Request-Id", HeaderValue::from_str(&request_id).unwrap());

    response
}
