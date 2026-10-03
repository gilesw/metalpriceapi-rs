# metalpriceapi

Unofficial async Rust client for [MetalpriceAPI](https://metalpriceapi.com/).

A focused client for latest rates, dated rates, daily OHLC, supported symbols
and account usage. It does not cover every provider endpoint. Requires Rust 1.85
or newer and an API subscription with access to the requested endpoints.

```toml
[dependencies]
metalpriceapi = "0.1"
```

The crate is being prepared for publication; until published, use a local path
dependency to this checkout.

```rust,no_run
use metalpriceapi::{Client, NaiveDate};

# async fn example() -> Result<(), metalpriceapi::Error> {
let client = Client::new("your-api-key");
let rates = client.latest("USD", &["XAU", "XAG"]).await?;
if let Some(price) = rates.price_in_base("XAU") {
    println!("Gold: {price} USD per provider metal unit");
}

let date = NaiveDate::from_ymd_opt(2025, 1, 28).unwrap();
let candle = client.ohlc("XAU", "USD", date).await?;
println!("Gold close: {} USD", candle.rate.close);
# Ok(())
# }
```

## Endpoint coverage

| Method | Endpoint | Result |
| --- | --- | --- |
| `latest(base, symbols)` | `/latest` | `Rates` |
| `historical(date, base, symbols)` | `/{date}` | `Rates` |
| `ohlc(base, currency, date)` | `/ohlc` | `Ohlc` |
| `symbols()` | `/symbols` | Symbol names |
| `usage()` | `/usage` | Plan and quota usage |

Dates use `chrono::NaiveDate`. An empty symbols slice omits the currencies filter.
`raw()` exposes generated operations, including the latest endpoint's optional
`math` parameter; these return envelopes whose `success` flag the caller must check.

## Rate direction, units and dates

For rates with base `USD`, `XAU` means gold units per USD. `USDXAU` means USD per
gold unit. `price_in_base("XAU")` reads the provider's `USDXAU` value. It returns
`None` when that entry is absent; it does not calculate a reciprocal or invent zero.

For OHLC, request base `XAU` and currency `USD` to obtain USD per gold unit.
The client preserves open, high, low and close directly. It does not invert prices
or swap high and low. Missing required prices are decoding errors.

Provider default units apply unless requested explicitly with
`client.with_unit(metalpriceapi::Unit::Gram)` (`TroyOz` and `Kilogram` are also
available). Unit selection and endpoints depend on the account plan. Confirm
units for each symbol before combining data from different providers.

Daily OHLC uses the provider's GMT calendar day. It is not an exchange-specific
closing auction price. Every price response retains its provider Unix timestamp;
callers decide whether it is sufficiently fresh. Prices are `f64`, with no
rounding or currency conversion by this client.

## Authentication and errors

The API key is sent in the `X-API-KEY` header. `Client` debug output omits it.
Supply credentials from your application's secret configuration. The example
accepts a key on standard input:

```sh
cargo run --example latest < /path/to/private-key-file
```

The US endpoint is the default. Select the EU endpoint with
`with_base_url(metalpriceapi::EU_BASE_URL)`. Custom roots must include `/v1`.
The generated client has a 30-second timeout and an 8 MiB response limit;
`with_max_response_body_bytes` changes the limit. There are no automatic retries.

Convenience methods return:

- `Error::Provider` for `success: false`, including HTTP 200 responses, preserving
  the provider code and message.
- `Error::Api` for unsuccessful HTTP responses, retaining status, response body
  and `Retry-After`.
- `Error::Decode` for malformed or incomplete successful responses.
- `Error::Transport` for transport and response-size failures.

`is_unauthorized()` and `is_rate_limited()` recognize HTTP statuses and documented
provider error codes. Provider response bodies can contain account details;
applications should decide what to expose to users.

## Generation and checks

Generated Rust is checked in; consumers do not need the generator.
Unlike providers that publish an OpenAPI file, this project maintains its own
small specification from the provider's documentation. It is not an official
MetalpriceAPI specification. See [specification provenance](specs/README.md).

```sh
mise run generate        # installs pinned openapi-to-rust 0.19.0 locally
mise run generate:check  # fail if checked-in output differs
mise run check           # release-script tests, fmt, Clippy, Rust tests, generation
```

Edit `specs/metalpriceapi.json`, then regenerate; do not edit `src/generated`
by hand. Fixtures cover documented responses and malformed/error cases.
Integration tests use local mock servers, without provider credentials.

On 2026-10-03, the Rust example also fetched and decoded live XAU/USD and
XAG/USD latest rates successfully using header authentication. Other endpoints
have not been live-verified.

See [RELEASING.md](RELEASING.md) for tag detection, dry runs and publishing.

## License

MIT OR Apache-2.0. This is an independent client, not endorsed by MetalpriceAPI.
API access and market-data use remain subject to the provider's terms.
