use warp::http::StatusCode;

pub async fn health_check_(
) -> Result<warp::http::Response<warp::http::StatusCode>, warp::http::Error> {
    Ok(warp::http::Response::builder().status(warp::http::StatusCode::OK))
}

// Result<warp::http::Response<std::string::String>, warp::http::Error>             argnarg
