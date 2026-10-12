# Changelog

## 0.2.0 — Unreleased

- Add the `timeframe` endpoint: daily rates for every requested symbol over a
  date range of up to 365 days, as `Timeframe` with `price_in_base(date, symbol)`.
- Add the `change` endpoint: start and end rates with the absolute and
  percentage change per symbol, as `Change`.
- Require Rust 1.98.1 and add GitHub Actions lint, test and release dry-run workflows.

## 0.1.0 — 2026-10-03

- Generated async client from a maintained OpenAPI subset: latest rates, dated
  rates, daily OHLC, symbols and account usage.
- Typed convenience methods, header authentication, explicit metal units and
  provider errors including HTTP 200 failure envelopes.
- Preserve rate direction, timestamps and missing values without implicit
  reciprocal calculations or zero-price defaults.
- Reproducible generation and tag-based release tasks through mise.
