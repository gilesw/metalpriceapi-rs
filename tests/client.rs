use metalpriceapi::{Client, Error, NaiveDate, Unit};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

fn fixture(name: &str) -> Value {
    let source = match name {
        "latest" => include_str!("fixtures/latest.json"),
        "historical" => include_str!("fixtures/historical.json"),
        "ohlc" => include_str!("fixtures/ohlc.json"),
        "symbols" => include_str!("fixtures/symbols.json"),
        "usage" => include_str!("fixtures/usage.json"),
        "error" => include_str!("fixtures/error.json"),
        _ => panic!("Unknown fixture"),
    };
    serde_json::from_str(source).unwrap()
}
fn client(server: &MockServer) -> Client {
    Client::new("test-secret").with_base_url(format!("{}/v1/", server.uri()))
}
fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2025, 1, 28).unwrap()
}
async fn respond(server: &MockServer, route: &str, body: Value) {
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn latest_preserves_rate_direction_and_uses_header_auth() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/latest"))
        .and(header("X-API-KEY", "test-secret"))
        .and(query_param("base", "USD"))
        .and(query_param("currencies", "XAU,XAG"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("latest")))
        .expect(1)
        .mount(&server)
        .await;
    let rates = client(&server)
        .latest("USD", &["XAU", "XAG"])
        .await
        .unwrap();
    assert_eq!(rates.base, "USD");
    assert_eq!(rates.timestamp, 1625609377);
    assert_eq!(rates.rates["XAU"], 0.00053853);
    assert_eq!(rates.price_in_base("XAU"), Some(1856.906765));
    assert_eq!(rates.price_in_base("MISSING"), None);
    let requests = server.received_requests().await.unwrap();
    assert!(!requests[0].url.as_str().contains("test-secret"));
    assert!(
        !requests[0]
            .url
            .query_pairs()
            .any(|(key, _)| key == "api_key" || key == "unit")
    );
}

#[tokio::test]
async fn historical_uses_calendar_date_and_explicit_unit() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/2025-01-28"))
        .and(query_param("base", "USD"))
        .and(query_param("currencies", "XAU"))
        .and(query_param("unit", "gram"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("historical")))
        .expect(1)
        .mount(&server)
        .await;
    let rates = client(&server)
        .with_unit(Unit::Gram)
        .historical(date(), "USD", &["XAU"])
        .await
        .unwrap();
    assert_eq!(
        rates.timestamp,
        fixture("historical")["timestamp"].as_i64().unwrap()
    );
}

#[tokio::test]
async fn ohlc_requests_quote_per_metal_without_inverting_high_and_low() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/ohlc"))
        .and(query_param("base", "XAU"))
        .and(query_param("currency", "USD"))
        .and(query_param("date", "2025-01-28"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("ohlc")))
        .expect(1)
        .mount(&server)
        .await;
    let ohlc = client(&server).ohlc("XAU", "USD", date()).await.unwrap();
    assert_eq!((ohlc.base.as_str(), ohlc.quote.as_str()), ("XAU", "USD"));
    assert_eq!(ohlc.timestamp, 1738108799);
    assert_eq!(ohlc.rate.close, 2742.2233288617);
    assert_eq!(ohlc.rate.high, 2764.9591200794);
    assert_eq!(ohlc.rate.low, 2735.1078165826);
    assert_eq!(ohlc.rate.open, 2741.9706872366);
}

#[tokio::test]
async fn missing_ohlc_field_is_an_error_not_zero() {
    let server = MockServer::start().await;
    let mut body = fixture("ohlc");
    body["rate"].as_object_mut().unwrap().remove("close");
    respond(&server, "/v1/ohlc", body).await;
    assert!(matches!(
        client(&server).ohlc("XAU", "USD", date()).await,
        Err(Error::Decode(_))
    ));
}

#[tokio::test]
async fn provider_failure_with_http_200_is_a_typed_error() {
    let server = MockServer::start().await;
    respond(&server, "/v1/latest", fixture("error")).await;
    let error = client(&server).latest("USD", &["XAU"]).await.unwrap_err();
    assert!(error.is_rate_limited());
    assert!(matches!(error, Error::Provider { code: Some(104), message } if !message.is_empty()));
}

#[tokio::test]
async fn provider_error_aliases_and_unknown_errors_are_preserved() {
    for (body, expected) in [
        (
            json!({"success":false,"error":{"statusCode":102,"message":"Invalid key"}}),
            Some(102),
        ),
        (
            json!({"success":false,"error":{"code":999,"info":"New failure"}}),
            Some(999),
        ),
        (json!({"success":false}), None),
    ] {
        let server = MockServer::start().await;
        respond(&server, "/v1/latest", body).await;
        let error = client(&server).latest("USD", &["XAU"]).await.unwrap_err();
        assert_eq!(error.is_unauthorized(), expected == Some(102));
        assert!(matches!(error, Error::Provider { code, .. } if code == expected));
    }
}

#[tokio::test]
async fn http_errors_preserve_status_body_and_retry_after() {
    for status in [401, 429, 503] {
        let server = MockServer::start().await;
        Mock::given(path("/v1/latest"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Retry-After", "60")
                    .set_body_string("upstream failure"),
            )
            .mount(&server)
            .await;
        let error = client(&server).latest("USD", &["XAU"]).await.unwrap_err();
        assert_eq!(error.is_rate_limited(), status == 429);
        assert_eq!(error.is_unauthorized(), status == 401);
        assert!(
            matches!(error, Error::Api { status: actual, body, retry_after } if actual == status && body == "upstream failure" && retry_after.as_deref() == Some("60"))
        );
    }
}

#[tokio::test]
async fn missing_success_fields_and_contradictory_envelopes_fail() {
    for field in ["base", "timestamp", "rates"] {
        let server = MockServer::start().await;
        let mut body = fixture("latest");
        body.as_object_mut().unwrap().remove(field);
        respond(&server, "/v1/latest", body).await;
        assert!(matches!(
            client(&server).latest("USD", &[]).await,
            Err(Error::Decode(_))
        ));
    }
    let server = MockServer::start().await;
    let mut body = fixture("latest");
    body["error"] = json!({"code":104,"info":"Failure"});
    respond(&server, "/v1/latest", body).await;
    assert!(matches!(
        client(&server).latest("USD", &[]).await,
        Err(Error::Decode(_))
    ));
}

#[tokio::test]
async fn empty_symbols_omits_filter_and_missing_reciprocal_stays_absent() {
    let server = MockServer::start().await;
    respond(
        &server,
        "/v1/latest",
        json!({"success":true,"base":"USD","timestamp":123,"rates":{"XAU":0.0005}}),
    )
    .await;
    let rates = client(&server).latest("USD", &[]).await.unwrap();
    assert_eq!(rates.price_in_base("XAU"), None);
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests[0]
            .url
            .query_pairs()
            .any(|(key, _)| key == "currencies")
    );
}

#[tokio::test]
async fn symbols_and_usage_decode_provider_envelopes() {
    let server = MockServer::start().await;
    respond(&server, "/v1/symbols", fixture("symbols")).await;
    respond(&server, "/v1/usage", fixture("usage")).await;
    assert_eq!(
        client(&server).symbols().await.unwrap()["AED"],
        "UAE Dirham"
    );
    let usage = client(&server).usage().await.unwrap();
    assert_eq!(usage.plan, "Business");
    assert_eq!((usage.used, usage.total, usage.remaining), (12, 100, 88));
}

#[tokio::test]
async fn every_convenience_endpoint_checks_provider_success() {
    let server = MockServer::start().await;
    for route in ["/v1/symbols", "/v1/usage", "/v1/ohlc", "/v1/2025-01-28"] {
        respond(&server, route, fixture("error")).await;
    }
    let client = client(&server);
    assert!(client.symbols().await.unwrap_err().is_rate_limited());
    assert!(client.usage().await.unwrap_err().is_rate_limited());
    assert!(
        client
            .ohlc("XAU", "USD", date())
            .await
            .unwrap_err()
            .is_rate_limited()
    );
    assert!(
        client
            .historical(date(), "USD", &["XAU"])
            .await
            .unwrap_err()
            .is_rate_limited()
    );
}

#[tokio::test]
async fn malformed_json_and_wrong_numeric_type_fail() {
    for body in [
        "not json",
        r#"{"success":true,"base":"USD","timestamp":123,"rates":{"XAU":"unknown"}}"#,
    ] {
        let server = MockServer::start().await;
        Mock::given(path("/v1/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server).latest("USD", &[]).await,
            Err(Error::Decode(_))
        ));
    }
}

#[tokio::test]
async fn response_size_limit_is_enforced() {
    let server = MockServer::start().await;
    respond(&server, "/v1/latest", fixture("latest")).await;
    assert!(matches!(
        client(&server)
            .with_max_response_body_bytes(16)
            .latest("USD", &[])
            .await,
        Err(Error::Transport(_))
    ));
}

#[test]
fn debug_does_not_disclose_credentials() {
    assert!(!format!("{:?}", Client::new("test-secret")).contains("test-secret"));
}
