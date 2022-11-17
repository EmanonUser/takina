use reqwest::blocking::{Client, Response};

pub fn get_ipv4() -> Option<Response> {
    let endpoint = "https://api4.ipify.org?format=txt";
    let client = Client::new();

    let res = client.get(endpoint).send();

    match res {
        Ok(r) => Some(r),
        Err(e) => {
            println!("Network error: Failed to fetch IPv4 addr");
            println!("{e}");
            None
        }
    }
}

pub fn get_ipv6() -> Option<Response> {
    let endpoint = "https://api6.ipify.org?format=txt";
    let client = Client::new();

    let res = client.get(endpoint).send();

    match res {
        Ok(r) => Some(r),
        Err(e) => {
            println!("Network error: Failed to fetch IPv6 addr");
            println!("{e}");
            None
        }
    }
}
