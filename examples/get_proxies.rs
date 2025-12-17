use std::{env, process};

use proxy_cfg::{ProxyConfig, get_proxy_config};
use url::Url;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        match get_proxy_config() {
            Ok(Some(ProxyConfig { proxies, .. })) => {
                for (_, p) in proxies {
                    println!("{}", p);
                }
            }
            Ok(None) => println!("No proxy configured"),
            Err(e) => {
                println!("Error getting proxies: {:?}", e);
                process::exit(1);
            }
        };
    } else {
        for arg in args {
            match get_proxy_config() {
                Ok(Some(proxy_config)) => match proxy_config.get_proxy_for_url(&Url::parse(&arg).unwrap()) {
                    Some(proxy) => println!("{} : {}", arg, proxy),
                    None => println!("No proxy needed for URL: '{}'", arg),
                },
                Ok(None) => println!("No proxy configured"),
                Err(e) => {
                    println!("Error getting proxies: {:?}", e);
                    process::exit(1);
                }
            }
        }
    }
}
