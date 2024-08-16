use super::*;
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn when_providing_name_it_should_return_the_appropriate_greeting() {
    let filter = create_http_filter();

    let response = request()
        .method("GET")
        .path("/api/glitch_guardian?name=Rust")
        .reply(&filter)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.body(),
        "Hello, Rust. This HTTP triggered function executed successfully."
    );
}

#[tokio::test]
async fn when_not_providing_name_it_should_return_the_default_greeting() {
    let filter = create_http_filter();

    let response = request()
        .method("GET")
        .path("/api/glitch_guardian")
        .reply(&filter)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
            response.body(),
            "This HTTP triggered function executed successfully. Pass a name in the query string for a personalized response."
        );
}

#[tokio::test]
async fn when_getting_wrong_path_it_should_return_not_found() {
    let filter = create_http_filter();

    let response = request()
        .method("GET")
        .path("/wrong/path")
        .reply(&filter)
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
