use std::io::{self, Read};
use std::fs::File;
use reqwest::{blocking::{Client, Response}, Error, StatusCode};
use serde_json;

use takina::GandiRecord;
use toml;
mod takina;
mod addr;

fn main() {
    
    let read_config_result = match read_config("./takina.toml") {
        Ok(s) => s,
        Err(e) => panic!("IO Error: Failed to read configuration file {e}")
    };

    let domains: takina::Config = parse_config(&read_config_result);

    for domain in &domains.domain {
        for record in domain.record() {
            record.validate_fields()
        };
    };
    
    let ipv4 = addr::get_ipv4()
        .expect("HTTP API Error: Unable to query for IPv4 addr")
        .text()
        .expect("Parse Error: Unable to get response test");

    let ipv6 = addr::get_ipv6()
        .expect("HTTP API Error: Unable to query for IPv4 addr")
        .text()
        .expect("Parse Error: Unable to get response test");

    
    for domain in &domains.domain {
        for record in domain.record() {

            let tmp = String::from("::1");
            let addr;

            if record.rtype() == "AAAA" {
                addr = &ipv6;
            } else if record.rtype() == "A" {
                addr = &ipv4;
            } else {
                addr = &tmp;
            }

            let gandi_query = match get_record(&domain, &record) {
                Ok(r) => r,
                Err(e) => {
                    println!("Network error: Unable to connect to gandi's API");
                    println!("{e}");
                    continue;
                }
            };

            let gandi_query = match gandi_query.status() {
                StatusCode::OK => gandi_query,
                StatusCode::NOT_FOUND => gandi_query,
                StatusCode::UNAUTHORIZED => {
                    println!("API HTTP Error: UNAUTHORIZED Bad authentication");
                    println!("{}", gandi_query.text().unwrap());
                    break;
                }
                StatusCode::FORBIDDEN => {
                    println!("API HTTP Error: FORBIDDEN Access to the resource is denied");
                    println!("{}", gandi_query.text().unwrap());
                    break;
                }
                _ => {
                    println!("API HTTP Error: UNEXPECTED status code see below");
                    println!("{}", gandi_query.text().unwrap());
                    continue;
                }
            };

            let record_state = match gandi_query.status() {
                StatusCode::OK => takina::State::DiffRecord,
                StatusCode::NOT_FOUND => takina::State::CreateRecord,
                _ => {
                    println!("API HTTP Error: UNEXPECTED status code see below");
                    println!("{}", gandi_query.text().unwrap());
                    continue;
                }
            };

            let api_state = match record_state {
                takina::State::DiffRecord => {
    
                    let grecord: GandiRecord = serde_json::from_str(
                        &gandi_query.text().expect("Reqwest Error: Failed to decode Response")
                    ).expect("Serde Error: Failed to Deserialze string");

                    if grecord.diff(&record, vec![addr.to_owned()]) {
                        let grecord = takina::GandiRecord {
                            rrset_values: vec![addr.to_owned()],
                            rrset_ttl: record.ttl(),
                            ..Default::default()
                        };
                        
                        println!("Info: Record Updated");
                        update_record(&domain, &record, &grecord)

                    } else  {
                        println!("Info: No Update Needed");
                        continue;
                    }
                
                },
                takina::State::CreateRecord => {
                    let grecord = takina::GandiRecord {
                        rrset_values: vec![addr.to_owned()],
                        rrset_ttl: record.ttl(),
                        ..Default::default()
                    };
                    println!("Info: New Record Created");
                    create_record(&domain, &record, &grecord)
                }
                _ => {
                    println!("UNEXPECTED Record State");
                    continue;
                }
            };

            match api_state {
                Ok(_) => continue,
                Err(e) => {
                    println!("API HTTP Error: UNEXPECTED status code see below");
                    println!("{}", e);
                    continue;
                }
            };
        }
    }
}


fn read_config(path: &str) -> Result<String, io::Error> {
    let mut f = File::open(path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

fn parse_config(conf: &String) -> takina::Config {
    match toml::from_str(&conf) {
        Ok(conf) => conf,
        Err(e) => panic!("Failed to parse toml configuration file {e}")
    }
}

fn get_record(domain: &takina::Domain, record: &takina::Record) -> Result<Response, Error> {
    let endpoint = format!(
        "https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        domain.name(),
        record.name(),
        record.rtype()
    );
    let client = Client::new();
    let res = client
        .get(endpoint)
        .header("Authorization", "Apikey ".to_owned() + &domain.api_key())
        .send()?;
    Ok(res)
}

fn update_record(domain: &takina::Domain, record: &takina::Record, grecord: &takina::GandiRecord) -> Result<Response, Error> {
    let endpoint = format!(
        "https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        domain.name(),
        record.name(),
        record.rtype()
    );
    let client = reqwest::blocking::Client::new();
    let res = client
        .put(&endpoint)
        .header("Authorization", "Apikey ".to_owned() + &domain.api_key())
        .body(serde_json::to_string(&grecord).unwrap())
        .send()?;
    Ok(res)
}

fn create_record(domain: &takina::Domain, record: &takina::Record, grecord: &takina::GandiRecord) -> Result<Response, Error> {
    let endpoint = format!(
        "https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        domain.name(),
        record.name(),
        record.rtype()
    );
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(&endpoint)
        .header("Authorization", "Apikey ".to_owned() + &domain.api_key())
        .body(serde_json::to_string(&grecord).unwrap())
        .send()?;
    Ok(res)
}