pub mod cloudflare;

#[derive(Debug)]
pub struct DnsUpdateResult {
    pub success: bool,
    pub message: String,
    pub record_id: Option<String>,
}
