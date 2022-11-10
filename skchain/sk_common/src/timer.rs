pub fn now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("System clock was before 1970.")
        .as_nanos()
}
