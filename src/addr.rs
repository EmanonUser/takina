use reqwest::{blocking::{Client, Response}, Error};

pub fn get_ipv4() -> Result<Response, Error> {
    let endpoint = "https://api4.ipify.org?format=txt";
    let client = Client::new();

    let res = client
        .get(endpoint)
        .send()?;
    Ok(res)    
}

pub fn get_ipv6() -> Result<Response, Error> {
    let endpoint = "https://api6.ipify.org?format=txt";
    let client = Client::new();

    let res = client
        .get(endpoint)
        .send()?;
    Ok(res)
}