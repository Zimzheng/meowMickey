pub mod app;
pub mod config;
pub mod timer;
pub mod window_state;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert_eq!(version(), "1.1.0");
    }
}
