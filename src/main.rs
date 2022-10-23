use reqwest::{Error, StatusCode};
use reqwest::blocking::Response as Response;
use std::env;
pub mod takina;

fn main() {
    let api_key = env::var("GANDI_API_KEY").expect("Gandi env var not set");
    let mut records: Vec<takina::Record> = Vec::new();
    let rustv6: takina::Record = takina::Record::new(
        String::from("rust"), 
        String::from("aaaa"),
        350,
        vec![get_ipv6()],
    );

    let rustv4: takina::Record = takina::Record::new(
        String::from("rust"), 
        String::from("a"),
        352,
        vec![get_ipv4()],
    );


    let rustvk: takina::Record = takina::Record::new(
        String::from("rustvk"), 
        String::from("a"),
        355,
        vec![get_ipv4(),],
    );

    records.push(rustv6);
    records.push(rustv4);
    records.push(rustvk);

    for record in &records {

        let gandi_query = match get_record(&api_key, &record) {
            Ok(r) => r,
            Err(e) => {
                println!("Network error: Unable to connect to gandi's API");
                println!("{e}");
                continue;
            }  
        };

        let query = match gandi_query.status() {
            StatusCode::OK => gandi_query,
            StatusCode::UNAUTHORIZED => {
                println!("API HTTP Error: UNAUTHORIZED Bad authentication");
                println!("{}", gandi_query.text().unwrap());
                continue; 
            }
            StatusCode::FORBIDDEN => {
                println!("API HTTP Error: FORBIDDEN Access to the resource is denied");
                println!("{}", gandi_query.text().unwrap());
                continue;
            }
            StatusCode::NOT_FOUND => {
                println!("API HTTP Error: NOT_FOUND record does not exist or bad url");
                match create_record(&api_key, &record) {
                    Ok(_) => {
                        println!("Info: Sucessfull creation for {}.{} type {}", record.get_name(), takina::DOMAIN_NAME, record.get_type());
                        println!("Info: New address(es) {:?}", record.get_values());
                        continue;
                    }
                    Err(e) => {
                        println!("API Error: Unable to update for {}.{} type {}", record.get_name(), takina::DOMAIN_NAME, record.get_type());
                        println!("{}", e);
                        continue;
                    }
                };

            }
            _ => {
                println!("API HTTP Error: UNEXPECTED status code see below");
                println!("{}", gandi_query.text().unwrap());
                continue;
            }
        };

        let res = query.text().expect("Parsing Error: Failed to process API response");
        let res: takina::Record = serde_json::from_str(&res).expect("Serde Error: Failed to Serialize API response");

        let status = match res.diff(&record) {
            takina::RecordState::Diff => {
                update_record(&api_key, &record)
            }
            takina::RecordState::Same => {
                println!("Info: No update needed for {}.{} type {}", record.get_name(), takina::DOMAIN_NAME, record.get_type());
                continue;
            }
        };

        match status {
            Ok(_) => {
                println!("Info: Sucessfull update for {}.{} type {}", record.get_name(), takina::DOMAIN_NAME, record.get_type());
                println!("Info: New address(es) {:?}", record.get_values());
            }
            Err(e) => {
                println!("API Error: Unable to update for {}.{} type {}", record.get_name(), takina::DOMAIN_NAME, record.get_type());
                println!("{}", e);
                continue;
            }
        }
    }
}

fn get_record(api_key: &String, record: &takina::Record) -> Result<Response, Error> {
    let endpoint = format!("https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}", takina::DOMAIN_NAME, record.get_name(), record.get_type());
    let client = reqwest::blocking::Client::new();
    let res = client.get(endpoint)
        .header("Authorization", "Apikey ".to_owned() + &api_key)
       .send()?;
    Ok(res)
}

fn update_record(api_key: &String, record: &takina::Record) -> Result<Response, Error> {
    let endpoint = format!("https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}", takina::DOMAIN_NAME, record.get_name(), record.get_type());
    let client = reqwest::blocking::Client::new();
    let res = client.put(&endpoint)
        .header("Authorization", "Apikey ".to_owned() + &api_key)
       .body(serde_json::to_string(&record).unwrap())
        .send()?;
    Ok(res)
}

fn create_record(api_key: &String, record: &takina::Record) -> Result<Response, Error> {
    let endpoint = format!("https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}", takina::DOMAIN_NAME, record.get_name(), record.get_type());
    let client = reqwest::blocking::Client::new();
    let res = client.post(&endpoint)
        .header("Authorization", "Apikey ".to_owned() + &api_key)
        .body(serde_json::to_string(&record).unwrap())
        .send()?;
    Ok(res)
}

fn get_ipv4() -> String {
    let endpoint = "https://api.ipify.org";
    let res = reqwest::blocking::get(endpoint)
        .expect("Failed to query API")
        .text()
        .expect("Failed to parse response");
    res
}

fn get_ipv6() -> String {
    // can return both IPv4 and IPv6 make sure an IPv6 is returned
    let endpoint = "https://api64.ipify.org";
    let res = reqwest::blocking::get(endpoint)
        .expect("Failed to query API")
        .text()
        .expect("Failed to parse response");
    res
}