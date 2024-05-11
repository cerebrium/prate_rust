use std::env;
use std::fs;

pub fn not_found() -> Result<warp::http::Response<std::string::String>, warp::http::Error> {
    warp::http::Response::builder()
        .status(warp::http::StatusCode::OK)
        .body(
            fs::read_to_string(env::current_dir().unwrap().join("./static/main.html"))
                .expect("404 404?"),
        )
}
