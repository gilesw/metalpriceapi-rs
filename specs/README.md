# Specification provenance

`metalpriceapi.json` is a project-authored OpenAPI 3.1 subset based on the
[provider documentation](https://metalpriceapi.com/documentation) and the
[official Python client](https://github.com/metalpriceapi/metalpriceapi-python),
reviewed on 2026-10-03. No official OpenAPI file was available from the locations
checked. Do not describe this file as provider-supplied or as a complete API schema.

It models latest, dated rates, OHLC, symbols and usage. Hourly/minutely rates,
conversion, timeframe, change and carat endpoints are outside this initial scope.
The specification and generated client are distributed under this project's
MIT OR Apache-2.0 licence.

The provider documents `X-API-KEY` header authentication as an alternative to
query-string authentication. This client uses the header.

Response envelopes require `success` but allow payload fields to be absent on
failure. The Rust convenience layer requires the payload fields on success and
turns provider-declared failures into errors. OHLC values are required individually.
Errors accept documented `code`/`info` and the `statusCode`/`message` shape handled
by an existing integration. Unknown codes and HTTP response bodies are retained.

`tests/fixtures` contains representative examples transcribed from the provider's
documentation, not captured account data. The symbols fixture is an excerpt.
These examples establish expected decoding; they do not prove live access to every
endpoint or every subscription tier.

Regenerate with `mise run generate` and verify with `mise run generate:check`.
