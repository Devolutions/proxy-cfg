
use super::*;
use std::env;

pub(crate) fn get_proxy_config() -> Result<ProxyConfig> {
    let vars: Vec<(String, String)> = env::vars().collect();
    let mut proxies = HashMap::new();
    let mut whitelist = HashSet::new();

    for (key, value) in vars {
        let key = key.to_lowercase();
        if key.ends_with("_proxy") {
            let scheme = &key[..key.len()-6];
            if scheme == "no" {
                for url in value.split(",").map(|s| s.trim()) {
                    if url.len() > 0 {
                        whitelist.insert(url.into());
                    }
                }
            } else {
                proxies.insert(scheme.into(), value);
            }
        }
    }

    if proxies.is_empty() {
        Err(NoProxyConfiguredError)
    } else {
        Ok(ProxyConfig {
            proxies,
            whitelist,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_basic() {
        env::set_var("HTTP_PROXY", "127.0.0.1");
        env::set_var("HTTPS_PROXY", "candybox2.github.io");
        env::set_var("FTP_PROXY", "http://9-eyes.com");
        env::set_var("NO_PROXY", "");

        let mut proxies = HashMap::new();
        proxies.insert("http".into(), "127.0.0.1".to_string());
        proxies.insert("https".into(), "candybox2.github.io".to_string());
        proxies.insert("ftp".into(), "http://9-eyes.com".to_string());

        let env_var_proxies = get_proxy_config().unwrap().proxies;
        if env_var_proxies.len() != 3 {
            // Other proxies are present on the host machine.
            for (k,..) in proxies.iter() {
                assert_eq!(env_var_proxies.get(k), proxies.get(k));
            }
        } else {
            assert_eq!(env_var_proxies, proxies);
        }
    }

    #[test]
    fn test_env_whitelist() {
        env::set_var("HTTP_PROXY", "127.0.0.1");
        env::set_var("HTTPS_PROXY", "candybox2.github.io");
        env::set_var("FTP_PROXY", "http://9-eyes.com");
        env::set_var("NO_PROXY", "google.com, 192.168.0.1, localhost, https://github.com/");

        assert_eq!(get_proxy_for_url(Url::parse("http://google.com").unwrap()).ok(), None);
        assert_eq!(get_proxy_for_url(Url::parse("https://localhost").unwrap()).ok(), None);
        assert_eq!(get_proxy_for_url(Url::parse("https://bitbucket.org").unwrap()).unwrap(), "candybox2.github.io");
    }
}
