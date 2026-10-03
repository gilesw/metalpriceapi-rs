# Changelog

## 0.1.0 — Unreleased

- Generated async client from a maintained OpenAPI subset: latest rates, dated
  rates, daily OHLC, symbols and account usage.
- Typed convenience methods, header authentication, explicit metal units and
  provider errors including HTTP 200 failure envelopes.
- Preserve rate direction, timestamps and missing values without implicit
  reciprocal calculations or zero-price defaults.
- Reproducible generation and tag-based release tasks through mise.
