pub struct Client {
    url: String,
}

impl Client {
    pub fn new(url: &str) -> Self {
        Self { url: url.to_string() }
    }
}
