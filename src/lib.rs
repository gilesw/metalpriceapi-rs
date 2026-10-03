//! Typed async client for MetalpriceAPI's rates, OHLC, symbols and usage endpoints.
//!
//! ```no_run
//! use metalpriceapi::Client;
//! # async fn example() -> Result<(), metalpriceapi::Error> {
//! let client = Client::new("your-api-key");
//! let rates = client.latest("USD", &["XAU", "XAG"]).await?;
//! println!("USD per gold unit: {:?}", rates.price_in_base("XAU"));
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod client;
mod error;

#[rustfmt::skip]
#[allow(dead_code, clippy::all, clippy::pedantic, missing_docs, rustdoc::all)]
pub mod generated {
    pub mod client;
    pub mod types;
}

pub use chrono::NaiveDate;
pub use client::{Client, EU_BASE_URL, Ohlc, Rates, US_BASE_URL};
pub use error::Error;
pub use generated::types::{OhlcValues, Unit, Usage};
