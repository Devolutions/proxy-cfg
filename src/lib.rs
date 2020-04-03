use std::collections::{HashMap, HashSet};

use url::Url;

#[cfg(windows)]
mod windows;

#[cfg(target_os="macos")]
mod macos;

#[cfg(feature = "env")]
mod env;

#[cfg(feature = "sysconfig_proxy")]
mod sysconfig_proxy;

mod errors;

use errors::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProxyConfig {
    pub proxies: HashMap<String, String>,
    pub whitelist: HashSet<String>,
    pub exclude_simple: bool,
    __other_stuff: (),
}

impl ProxyConfig {
/// Returns the proxy to use for the given URL
    pub fn get_proxy_for_url(&self, url: Url) -> Option<String> {
        // TODO Pattern match
        if self.whitelist.contains(&url.host_str().unwrap_or_default().to_lowercase()) {
            return None
        }

        self.proxies.get(url.scheme()).map(|s| s.to_string())
    }
}

type ProxyFn = fn() -> Result<ProxyConfig>;

const METHODS: &[&ProxyFn] = &[
    #[cfg(feature = "env")]
    &(env::get_proxy_config as ProxyFn),
    #[cfg(feature = "sysconfig_proxy")]
    &(sysconfig_proxy::get_proxy_config as ProxyFn), //This configurator has to come after the `env` configurator, because environment variables take precedence over /etc/sysconfig/proxy
    #[cfg(windows)]
    &(windows::get_proxy_config as ProxyFn),
    #[cfg(target_os="macos")]
    &(macos::get_proxy_config as ProxyFn),
];

pub fn get_proxy_config() -> Result<ProxyConfig> {
    let mut last_err = Error::PlatformNotSupported;
    for get_proxy_config in METHODS {
        match get_proxy_config() {
            Ok(config) => return Ok(config),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test_get_proxies() {
        let _ = get_proxy_config();
    }

    #[test]
    fn smoke_test_get_proxy_for_url() {
        let proxy_config = get_proxy_config().unwrap();
        let _ = proxy_config.get_proxy_for_url(Url::parse("https://google.com").unwrap());
    }
}
