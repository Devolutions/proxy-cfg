
use super::*;
use std::env;

pub(crate) fn get_proxy_config() -> Result<Option<ProxyConfig>> {
    let vars: Vec<(String, String)> = env::vars().collect();
    let mut proxy_config: ProxyConfig = Default::default();

    for (key, value) in vars {
        let key = key.to_lowercase();
        if key.ends_with("_proxy") {
            let scheme = &key[..key.len() - 6];
            if scheme == "no" {
                for url in value.split(",").map(|s| s.trim()) {
                    if url.len() > 0 {
                        proxy_config.whitelist.insert(url.to_string().to_lowercase());
                    }
                }
            } else {
                proxy_config.proxies.insert(scheme.into(), value);
            }
        }
    }

    if proxy_config.proxies.len() == 0 {
        return Ok(None)
    }

    Ok(Some(proxy_config))
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

        let env_var_proxies = get_proxy_config().unwrap().unwrap().proxies;
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

        let proxy_config = get_proxy_config().unwrap().unwrap();

        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("http://google.com").unwrap()), None);
        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("https://localhost").unwrap()), None);
        assert_eq!(proxy_config.get_proxy_for_url(&Url::parse("https://bitbucket.org").unwrap()).unwrap(), "candybox2.github.io");
    }
}
