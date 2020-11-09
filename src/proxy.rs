use thiserror::*;
use url::Url;
use warp::filters::path::FullPath;
use warp::http::{self, HeaderMap, HeaderValue, Method};
use warp::hyper::body::Bytes;
use warp::{Filter, Rejection};

#[derive(Error, Debug)]
pub enum ForwardError {
    #[error("return bytes failed {0}")]
    ReturnBytes(#[from] reqwest::Error),
    #[error("http response failed {0}")]
    HttpResponse(warp::http::Error),
    #[error("fail building request {0}")]
    FailBuildingQuery(reqwest::Error),
}

impl warp::reject::Reject for ForwardError {}


pub fn api() -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    let v1 = warp::path("v1");
    let v2 = warp::path("v2");
    let v3 = warp::path("v3");
    use std::str::FromStr;

    let proxy_address_v1 = Url::from_str("http://localhost:3001").unwrap();
    let proxy_address_v2 = Url::from_str("http://localhost:3002").unwrap();
    let proxy_address_v3 = Url::from_str("http://localhost:3003").unwrap();
    v1.and(forward(proxy_address_v1).with(warp::log("v1")))
        .or(v2.and(forward(proxy_address_v2)).with(warp::log("v2")))
        .or(v3.and(forward(proxy_address_v3)).with(warp::log("v3")))
        .boxed()
}

pub fn forward(
    proxy_address: Url,
) -> impl Filter<Extract = (warp::http::Response<Bytes>,), Error = Rejection> + Clone {
    let proxy_address = warp::any().map(move || proxy_address.clone());

    let data_filter = warp::path::full()
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes());

    proxy_address
        .and(data_filter)
        .and_then(forward_and_wait)
        .boxed()
}

async fn forward_and_wait(
    proxy_address: Url,
    uri: FullPath,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Result<http::Response<Bytes>, Rejection> {
    let mut dest_url = proxy_address.clone();
    dest_url.set_path(uri.as_str());

    let client = reqwest::Client::new();
    let request = client
        .request(method, dest_url)
        .headers(headers)
        .body(body)
        .build()
        .map_err(|e| warp::reject::custom(ForwardError::FailBuildingQuery(e)))?;

    let response = client.execute(request).await;
    match response {
        Err(e) => {
            let content = Bytes::from(e.to_string());
            let builder = http::Response::builder();
            builder
                .status(500)
                .body(content)
                .map_err(|e| warp::reject::custom(ForwardError::HttpResponse(e)))
        }
        Ok(response) => {
            let mut builder = http::Response::builder();

            let status = response.status();

            // copy all headers
            for (k, v) in response.headers().iter() {
                builder = builder.header(k, v);
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|e| warp::reject::custom(ForwardError::ReturnBytes(e)))?;

            builder
                .status(status)
                .body(bytes)
                .map_err(|e| warp::reject::custom(ForwardError::HttpResponse(e)))
        }
    }
}
