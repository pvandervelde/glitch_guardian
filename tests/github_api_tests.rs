#[tokio::test]
async fn test_issue_creation() {
    let mut server = mockito::Server::new();

    let mock = server.mock("POST", "/project/columns/cards")
        .with_status(201)
        .with_body(r#"("id": 1"#)
        .create();

    std::env::set_var("GITHUB_API_URL", &server.url());

    let result = github_webhook(simulated_payload()).await();

    assert_eq!(result.status(), 200);
    mock.assert();
}
