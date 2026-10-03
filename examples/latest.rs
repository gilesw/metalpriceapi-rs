use metalpriceapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::io::read_to_string(std::io::stdin())?;
    let rates = Client::new(api_key.trim())
        .latest("USD", &["XAU", "XAG"])
        .await?;
    println!("{}", serde_json::to_string_pretty(&rates)?);
    Ok(())
}
