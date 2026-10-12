use std::collections::BTreeMap;

use crate::generated::{
    client::HttpClient,
    types::{ApiFailure, ChangeValues, RatesResponse},
};
use crate::{Error, NaiveDate, OhlcValues, Unit, Usage};

pub const US_BASE_URL: &str = "https://api.metalpriceapi.com/v1";
pub const EU_BASE_URL: &str = "https://api-eu.metalpriceapi.com/v1";

/// Rates exactly as returned by the provider, with their data timestamp.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Rates {
    pub base: String,
    pub timestamp: i64,
    pub rates: BTreeMap<String, f64>,
}

impl Rates {
    /// Provider-supplied reciprocal rate: base currency per unit of `symbol`.
    /// For base USD and symbol XAU this reads USDXAU, not XAU. No unit conversion
    /// or fallback inversion is performed. An absent rate remains absent.
    pub fn price_in_base(&self, symbol: &str) -> Option<f64> {
        self.rates.get(&format!("{}{symbol}", self.base)).copied()
    }
}

impl TryFrom<RatesResponse> for Rates {
    type Error = Error;

    fn try_from(response: RatesResponse) -> Result<Self, Error> {
        check_success(response.success, response.error)?;
        Ok(Self {
            base: required(response.base, "base")?,
            timestamp: required(response.timestamp, "timestamp")?,
            rates: required(response.rates, "rates")?.additional_properties,
        })
    }
}

/// Daily rates over a date range, exactly as returned, keyed by calendar date.
/// Days the provider leaves out are absent, never filled in.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Timeframe {
    pub base: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub rates: BTreeMap<NaiveDate, BTreeMap<String, f64>>,
}

impl Timeframe {
    /// Provider-supplied reciprocal rate for one day: base currency per unit of
    /// `symbol`, read from the `{base}{symbol}` entry. Absent stays absent.
    pub fn price_in_base(&self, date: NaiveDate, symbol: &str) -> Option<f64> {
        self.rates
            .get(&date)?
            .get(&format!("{}{symbol}", self.base))
            .copied()
    }
}

/// Start and end rates with the change per symbol, in the provider's rate
/// direction (units of `symbol` per base) without inversion.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Change {
    pub base: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub rates: BTreeMap<String, ChangeValues>,
}

/// OHLC in quote units per base unit, without inversion or unit conversion.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Ohlc {
    pub base: String,
    pub quote: String,
    pub timestamp: i64,
    pub rate: OhlcValues,
}

/// Sends the API key in the X-API-KEY header.
#[derive(Clone)]
pub struct Client {
    inner: HttpClient,
    unit: Option<Unit>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("unit", &self.unit)
            .finish_non_exhaustive()
    }
}

impl Client {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            inner: HttpClient::new().with_api_key(api_key),
            unit: None,
        }
    }

    /// Override the complete API root, including /v1. Also useful for mock servers.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        self.inner = self.inner.with_base_url(base_url.trim_end_matches('/'));
        self
    }

    /// Request an explicit metal unit where the account plan permits it.
    /// Without this option the provider's default units apply.
    pub fn with_unit(mut self, unit: Unit) -> Self {
        self.unit = Some(unit);
        self
    }

    pub fn with_max_response_body_bytes(mut self, limit: usize) -> Self {
        self.inner = self.inner.with_max_response_body_bytes(limit);
        self
    }

    /// Low-level operations return provider envelopes; callers must inspect
    /// success themselves. Convenience methods turn success:false into errors.
    pub fn raw(&self) -> &HttpClient {
        &self.inner
    }

    /// Latest rates. An empty symbols list requests the provider's default set.
    pub async fn latest(&self, base: &str, symbols: &[&str]) -> Result<Rates, Error> {
        self.inner
            .get_latest(
                Some(base),
                symbol_list(symbols),
                self.unit.clone(),
                None::<&str>,
            )
            .await?
            .try_into()
    }

    /// Rates for a calendar date, preserving the provider's historical timestamp.
    pub async fn historical(
        &self,
        date: NaiveDate,
        base: &str,
        symbols: &[&str],
    ) -> Result<Rates, Error> {
        self.inner
            .get_historical(
                date.to_string(),
                Some(base),
                symbol_list(symbols),
                self.unit.clone(),
            )
            .await?
            .try_into()
    }

    /// Daily rates for each calendar day from `start` to `end` inclusive. The
    /// provider allows at most 365 days per request.
    pub async fn timeframe(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        base: &str,
        symbols: &[&str],
    ) -> Result<Timeframe, Error> {
        let response = self
            .inner
            .get_timeframe(
                Some(base),
                symbol_list(symbols),
                start.to_string(),
                end.to_string(),
                self.unit.clone(),
            )
            .await?;
        check_success(response.success, response.error)?;
        let mut rates = BTreeMap::new();
        for (day, daily) in required(response.rates, "rates")?.additional_properties {
            rates.insert(parse_date(&day)?, daily.additional_properties);
        }
        Ok(Timeframe {
            base: required(response.base, "base")?,
            start_date: parse_date(&required(response.start_date, "start_date")?)?,
            end_date: parse_date(&required(response.end_date, "end_date")?)?,
            rates,
        })
    }

    /// Rate change per symbol between two calendar days.
    pub async fn change(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        base: &str,
        symbols: &[&str],
    ) -> Result<Change, Error> {
        let response = self
            .inner
            .get_change(
                Some(base),
                symbol_list(symbols),
                start.to_string(),
                end.to_string(),
                self.unit.clone(),
            )
            .await?;
        check_success(response.success, response.error)?;
        Ok(Change {
            base: required(response.base, "base")?,
            start_date: parse_date(&required(response.start_date, "start_date")?)?,
            end_date: parse_date(&required(response.end_date, "end_date")?)?,
            rates: required(response.rates, "rates")?.additional_properties,
        })
    }

    /// Daily OHLC in quote units per base unit. For USD per unit of gold, use
    /// base XAU and currency USD. The provider uses a GMT calendar day.
    pub async fn ohlc(&self, base: &str, currency: &str, date: NaiveDate) -> Result<Ohlc, Error> {
        let response = self
            .inner
            .get_ohlc(Some(base), currency, date.to_string(), self.unit.clone())
            .await?;
        check_success(response.success, response.error)?;
        Ok(Ohlc {
            base: required(response.base, "base")?,
            quote: required(response.quote, "quote")?,
            timestamp: required(response.timestamp, "timestamp")?,
            rate: required(response.rate, "rate")?,
        })
    }

    pub async fn symbols(&self) -> Result<BTreeMap<String, String>, Error> {
        let response = self.inner.get_symbols().await?;
        check_success(response.success, response.error)?;
        Ok(required(response.symbols, "symbols")?.additional_properties)
    }

    pub async fn usage(&self) -> Result<Usage, Error> {
        let response = self.inner.get_usage().await?;
        check_success(response.success, response.error)?;
        required(response.result, "result")
    }
}

fn symbol_list(symbols: &[&str]) -> Option<String> {
    (!symbols.is_empty()).then(|| symbols.join(","))
}

fn parse_date(text: &str) -> Result<NaiveDate, Error> {
    text.parse()
        .map_err(|_| Error::Decode(format!("Response date is not YYYY-MM-DD: {text}")))
}

fn required<T>(value: Option<T>, field: &str) -> Result<T, Error> {
    value.ok_or_else(|| Error::Decode(format!("Successful response is missing {field}")))
}

fn check_success(success: bool, error: Option<ApiFailure>) -> Result<(), Error> {
    if !success {
        let error = error.unwrap_or_default();
        return Err(Error::Provider {
            code: error.code.or(error.status_code),
            message: error
                .info
                .or(error.message)
                .unwrap_or_else(|| "Provider reported failure without details".into()),
        });
    }
    if error.is_some() {
        return Err(Error::Decode(
            "Response contains both success:true and an error".into(),
        ));
    }
    Ok(())
}
