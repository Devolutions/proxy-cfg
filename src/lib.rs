use std::collections::{HashMap, HashSet};

use url::Url;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(feature = "env")]
mod env;

#[cfg(feature = "sysconfig_proxy")]
mod sysconfig_proxy;

mod errors;

use errors::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProxyConfig {
    pub proxies: HashMap<String, String>,
    pub whitelist: HashSet<String>,
    pub exclude_simple: bool,
}

impl ProxyConfig {
    pub fn use_proxy_for_address(&self, address: &str) -> bool {
        // Extract and normalize the hostname.
        let host = if let Ok(url) = Url::parse(address)
            && let Some(url_host) = url.host_str()
        {
            url_host.to_lowercase()
        } else {
            address.to_lowercase()
        };

        // Check if simple hostnames (no dots) should bypass the proxy.
        if self.exclude_simple && !host.contains('.') {
            return false;
        }

        // Check exact hostname match in whitelist.
        if self.whitelist.contains(host.as_str()) {
            return false;
        }

        // Check wildcard suffix matches (e.g., "*.example.com" matches "sub.example.com").
        // TODO: Wildcard matches on IP address, e.g. 192.168.*.*
        // TODO: Subnet matches on IP address, e.g. 192.168.16.0/24
        if self.whitelist.iter().any(|pattern| {
            if let Some(pos) = pattern.rfind('*') {
                let suffix = &pattern[pos + 1..];
                !suffix.is_empty() && host.ends_with(suffix)
            } else {
                false
            }
        }) {
            return false;
        }

        true
    }

    pub fn get_proxy_for_url(&self, url: &Url) -> Option<String> {
        match self.use_proxy_for_address(url.as_str()) {
            true => self
                .proxies
                .get(url.scheme())
                .or_else(|| self.proxies.get("*"))
                .map(|s| s.to_lowercase()), // FIXME: URL is case sensitive
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
    #[cfg(target_os = "macos")]
    &(macos::get_proxy_config as ProxyFn),
];

pub fn get_proxy_config() -> Result<Option<ProxyConfig>> {
    #[allow(clippy::const_is_empty)]
    if METHODS.is_empty() {
        return Err(Error::PlatformNotSupported);
    }

    let mut last_err: Option<Error> = None;
    for get_proxy_config in METHODS {
        match get_proxy_config() {
            Ok(Some(config)) => return Ok(Some(config)),
            Err(e) => last_err = Some(e),
            _ => {}
        }
    }

    if let Some(e) = last_err {
        return Err(e);
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use url::Url;

    use super::{ProxyConfig, get_proxy_config};

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
            proxies: map! {
                "http".into() => "1.1.1.1".into(),
                "https".into() => "2.2.2.2".into()
            },
            whitelist: vec!["www.devolutions.net", "*.microsoft.com", "*apple.com"]
                .into_iter()
                .map(|s| s.to_owned())
                .collect(),
            exclude_simple: true,
            ..Default::default()
        };

        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://simpledomain").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://simple.domain").unwrap()),
            Some("1.1.1.1".into())
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://www.devolutions.net").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://www.microsoft.com").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://www.microsoft.com.fun").unwrap()),
            Some("1.1.1.1".into())
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://test.apple.com").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("https://test.apple.net").unwrap()),
            Some("2.2.2.2".into())
        );
    }

    #[test]
    fn test_wildcard_matching_edge_cases() {
        let proxy_config = ProxyConfig {
            proxies: map! {
                "http".into() => "1.1.1.1".into()
            },
            whitelist: vec![
                "*test*.com",        // Multiple asterisks.
                "*.sub.example.com", // Wildcard at start.
                "*",                 // Single asterisk (should match everything after it, which is empty).
                "foo*",              // Wildcard at end.
                "*.org",             // Simple wildcard domain.
            ]
            .into_iter()
            .map(|s| s.to_owned())
            .collect(),
            exclude_simple: false,
            ..Default::default()
        };

        // Test multiple asterisks: should use the last asterisk and match ".com" suffix.
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://example.com").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://whatever.com").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://notmatching.net").unwrap()),
            Some("1.1.1.1".into())
        );

        // Test *.sub.example.com pattern.
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://foo.sub.example.com").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://sub.example.com").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://example.com").unwrap()),
            None // Already matched by "*test*.com" -> "*.com".
        );

        // Test single asterisk with nothing after it (empty suffix - should not match).
        // Since suffix is empty, !suffix.is_empty() is false, so this pattern shouldn't bypass.
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://anything.xyz").unwrap()),
            Some("1.1.1.1".into())
        );

        // Test wildcard at end "foo*" - matches hosts ending with empty string after the *.
        // rfind('*') finds the asterisk, suffix is "", !suffix.is_empty() is false.
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://foobar.xyz").unwrap()),
            Some("1.1.1.1".into())
        );

        // Test *.org pattern.
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://example.org").unwrap()),
            None
        );
        assert_eq!(
            proxy_config.get_proxy_for_url(&Url::parse("http://test.example.org").unwrap()),
            None
        );
    }

    #[test]
    fn test_use_proxy_for_address_case_insensitivity() {
        let proxy_config = ProxyConfig {
            proxies: map! {
                "http".into() => "1.1.1.1".into()
            },
            // Whitelist patterns are normalized to lowercase (as done by all platform-specific code).
            whitelist: vec!["*.example.com", "test.local"]
                .into_iter()
                .map(|s| s.to_owned())
                .collect(),
            exclude_simple: false,
            ..Default::default()
        };

        // Hostnames should be matched case-insensitively (normalized to lowercase internally).
        assert!(!proxy_config.use_proxy_for_address("http://sub.example.com"));
        assert!(!proxy_config.use_proxy_for_address("http://SUB.EXAMPLE.COM"));
        assert!(!proxy_config.use_proxy_for_address("http://Sub.Example.Com"));
        assert!(!proxy_config.use_proxy_for_address("http://test.local"));
        assert!(!proxy_config.use_proxy_for_address("http://TEST.LOCAL"));
        assert!(!proxy_config.use_proxy_for_address("http://Test.Local"));
        assert!(proxy_config.use_proxy_for_address("http://other.domain"));
    }

    #[test]
    fn test_exclude_simple_hostnames() {
        let proxy_config = ProxyConfig {
            proxies: map! {
                "http".into() => "1.1.1.1".into()
            },
            whitelist: HashSet::new(),
            exclude_simple: true,
            ..Default::default()
        };

        // Simple hostnames (no dots) should bypass proxy.
        assert!(!proxy_config.use_proxy_for_address("http://localhost"));
        assert!(!proxy_config.use_proxy_for_address("http://intranet"));

        // Hostnames with dots should use proxy.
        assert!(proxy_config.use_proxy_for_address("http://example.com"));
        assert!(proxy_config.use_proxy_for_address("http://sub.example.com"));
    }
}
