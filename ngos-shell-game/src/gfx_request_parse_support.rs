pub fn parse_driver_request_id(header: &str) -> Option<u64> {
    let request = header.strip_prefix("request:")?;
    let end = request.find(' ').unwrap_or(request.len());
    request[..end].parse::<u64>().ok()
}
