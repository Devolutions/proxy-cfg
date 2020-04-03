// extern crate proxy_cfg;
// extern crate url;
// use url::Url;

pub fn main () {
    // match proxy_cfg::get_proxy_for_url(Url::parse("https://google.com").unwrap()).ok(){
    //     Some(a) => {
    //         println!("Proxy for google.com: {}", a);
    //     },
    //     None => {
    //         println!("https://google.com can be accessed without a proxy");
    //     },
    // };
    // match proxy_cfg::get_proxy_config() {
    //     Ok(proxy_cfg::ProxyConfig { proxies, whitelist, .. }) => {
    //         let mut i = 0;
    //         for p in proxies {
    //             println!("Proxy: {:?}, Exceptions: {:?}", p, whitelist[i]);
    //             i=i+1;
    //         }
    //     },
    //     Err(e) => {
    //         println!("Error getting proxies: {:?}", e);
    //     },
    // };
}