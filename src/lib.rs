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
    pub fn use_proxy_for_host(&self, host: &str) -> bool {
        let host = host.to_lowercase();

        if self.exclude_simple && !host.chars().any(|c| c == '.') {
            return false
        }

        if self.whitelist.contains(&host) {
            return false
        }

        // TODO: Wildcard matches on IP address, e.g. 192.168.*.*
        // TODO: Subnet matches on IP address, e.g. 192.168.16.0/24

        if self.whitelist.iter().any(|s| {
            if let Some(pos) = s.rfind('*') {
                let slice = &s[pos + 1..];
                return slice.len() > 0 && host.ends_with(slice)
            }
            false 
        }) { return false }

        true
    }

    pub fn get_proxy_for_url(&self, url: &Url) -> Option<String> {
        let host = match url.host_str() {
            Some(host) => host.to_lowercase(),
            None => return None,
        }; 

        match self.use_proxy_for_host(&host) {
            true => self.proxies.get(url.scheme()).map(|s| s.to_string().to_lowercase()),
            false => None,
        }
    }
}

type ProxyFn = fn() -> Result<Option<ProxyConfig>>;

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

pub fn get_proxy_config() -> Result<Option<ProxyConfig>> {
    if METHODS.len() == 0 {
        return Err(Error::PlatformNotSupported)
    }

    let mut last_err: Option<Error> = None;
    for get_proxy_config in METHODS {
        match get_proxy_config() {
            Ok(Some(config)) => return Ok(Some(config)),
            Err(e) => last_err = Some(e),
            _ => {},
        }
    }

    if let Some(e) = last_err {
        return Err(e)
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! map(
        { $($key:expr => $value:expr),+ } => {
            {
                let mut m = ::std::collections::HashMap::new();
                $(
                    m.insert($key, $value);
                )+
                m
            }
         };
    );

    #[test]
    fn smoke_test_get_proxies() {
        let _ = get_proxy_config();
    }

    #[test]
    fn smoke_test_get_proxy_for_url() {
        if let Some(proxy_config) = get_proxy_config().unwrap() {
            let _ = proxy_config.get_proxy_for_url(&Url::parse("https://google.com").unwrap());
        }
    }

    #[test]
    fn test_get_proxy_for_url() {
        let proxy_config = ProxyConfig { 
            proxies: map!{ 
                "http".into() => "1.1.1.1".into(), 
                "https".into() => "2.2.2.2".into() 
            },
            whitelist: vec![
                "www.devolutions.net", 
                "*.microsoft.com", 
                "*apple.com"
            ].into_iter().map(|s| s.to_string()).collect(),
            exclude_simple: true,
            ..Default::default() 
        };

        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("http://simpledomain").unwrap()), None);
        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("http://simple.domain").unwrap()), Some("1.1.1.1".into()));
        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("http://www.devolutions.net").unwrap()), None);
        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("http://www.microsoft.com").unwrap()), None);
        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("http://www.microsoft.com.fun").unwrap()), Some("1.1.1.1".into()));
        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("http://test.apple.com").unwrap()), None);
        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("https://test.apple.net").unwrap()), Some("2.2.2.2".into()));
    }
}
