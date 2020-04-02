extern crate url;

use std::collections::HashMap;

use url::Url;

#[cfg(windows)]
mod windows;

#[cfg(target_os="macos")]
pub mod macos;
#[cfg(target_os="macos")]
extern crate plist;

#[cfg(feature = "env")]
mod env;

#[cfg(feature = "sysconfig_proxy")]
mod sysconfig_proxy;

mod errors;
mod util;

pub use errors::ProxyConfigError;
use errors::*;
use errors::ProxyConfigError::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProxyConfig {
    pub proxies: HashMap<String, Url>,
    pub whitelist: Vec<String>,
    __other_stuff: (),
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

/// Returns a vector of URLs for the proxies configured by the system
pub fn get_proxy_config() -> Result<ProxyConfig> {
    let mut last_err = PlatformNotSupportedError;
    for get_proxy_config in METHODS {
        match get_proxy_config() {
            Ok(config) => return Ok(config),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// Returns the proxy to use for the given URL
/// TODO: Note that Windows bypass list can specify '<local>' for all intranet addresses
pub fn get_proxy_for_url(url: Url) -> Result<Url> {
    // TODO: cache get_proxy_config result?
    match get_proxy_config() {
        Ok(config) => {
            for domain in config.whitelist {
                // TODO: make this more robust
                if url.domain().unwrap().eq_ignore_ascii_case(&domain) {
                    return Err(NoProxyNeededError);
                }
            }

            if let Some(url) = config.proxies.get(url.scheme()) {
                Ok(url.clone())
            } else {
                Err(NoProxyForSchemeError(url.scheme().to_string()))
            }
        },
        Err(e) => Err(e),
    }
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
        let _ = get_proxy_for_url(Url::parse("https://google.com").unwrap());
    }
}
