use reqwest::{
    blocking::{Client, Response},
    Error, StatusCode,
};
use std::io;

use takina::GandiRecord;
mod addr;
mod takina;

fn main() {
    let read_config_result = match read_config("./takina.toml") {
        Ok(s) => s,
        Err(e) => panic!("IO Error: Failed to read configuration file {e}"),
    };

    let domains: takina::Config = parse_config(&read_config_result);

    for domain in &domains.domain {
        for record in domain.record() {
            record.validate_fields()
        }
    }

    let mut disable_ipv4 = false;
    let ipv4 = match addr::get_ipv4() {
        Some(r) => r.text().expect("API Error: Unable to parse response test"),
        None => {
            disable_ipv4 = true;
            String::from("127.0.0.1")
        }
    };

    let mut disable_ipv6 = false;
    let ipv6 = match addr::get_ipv6() {
        Some(r) => r.text().expect("API Error: Unable to parse response test"),
        None => {
            disable_ipv6 = true;
            String::from("::1")
        }
    };

    for domain in &domains.domain {
        for record in domain.record() {
            let addr;

            if record.rtype() == "AAAA" && !disable_ipv6 {
                addr = &ipv6;
            } else if record.rtype() == "AAAA" && disable_ipv6 {
                println!(
                    "Info: Skiping {}.{} Type: {}",
                    record.name(),
                    domain.name(),
                    record.rtype(),
                );
                continue;
            } else if record.rtype() == "A" && !disable_ipv4 {
                addr = &ipv4;
            } else if record.rtype() == "A" && disable_ipv4 {
                println!(
                    "Info: Skiping {}.{} Type: {}",
                    record.name(),
                    domain.name(),
                    record.rtype(),
                );
                continue;
            } else {
                panic!("State Error: Unexpected program state");
            }

            let gandi_network_response = get_record(domain, record);
            let gandi_api_response = match gandi_network_response {
                Ok(r) => r,
                Err(e) => {
                    println!("Network error: Unable to connect to gandi's API");
                    println!("{e}");
                    continue;
                }
            };

            let gandi_api_response = match gandi_api_response.status() {
                StatusCode::OK => gandi_api_response,
                StatusCode::NOT_FOUND => gandi_api_response,
                StatusCode::UNAUTHORIZED => {
                    println!(
                        "API HTTP Error: Bad authentication attempt because of a wrong API Key."
                    );
                    println!("{}", gandi_api_response.text().unwrap());
                    break;
                }
                StatusCode::FORBIDDEN => {
                    println!("API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it.");
                    println!("{}", gandi_api_response.text().unwrap());
                    break;
                }
                _ => {
                    println!("API HTTP Error: UNEXPECTED status code see below");
                    println!("{}", gandi_api_response.text().unwrap());
                    continue;
                }
            };

            let record_state = match gandi_api_response.status() {
                StatusCode::OK => takina::State::DiffRecord,
                StatusCode::NOT_FOUND => takina::State::CreateRecord,
                _ => {
                    println!("API HTTP Error: UNEXPECTED status code see below");
                    println!("{}", gandi_api_response.text().unwrap());
                    continue;
                }
            };

            let gandi_network_response = match record_state {
                takina::State::DiffRecord => {
                    let grecord: GandiRecord = serde_json::from_str(
                        &gandi_api_response
                            .text()
                            .expect("Reqwest Error: Failed to decode Response"),
                    )
                    .expect("Serde Error: Failed to Deserialze string");

                    if grecord.diff(record, vec![addr.to_owned()]) {
                        let grecord = takina::GandiRecord {
                            rrset_values: vec![addr.to_owned()],
                            rrset_ttl: record.ttl(),
                            ..Default::default()
                        };
                        let res = update_record(domain, record, &grecord);
                        println!(
                            "Info: Record Updated, {}.{} IpAddr: {} TTL: {}",
                            record.name(),
                            domain.name(),
                            addr,
                            record.ttl(),
                        );
                        res
                    } else {
                        println!(
                            "Info: No Update Needed for {}.{} IpAddr: {} TTL: {}",
                            record.name(),
                            domain.name(),
                            addr,
                            record.ttl(),
                        );
                        continue;
                    }
                }
                takina::State::CreateRecord => {
                    let grecord = takina::GandiRecord {
                        rrset_values: vec![addr.to_owned()],
                        rrset_ttl: record.ttl(),
                        ..Default::default()
                    };

                    create_record(domain, record, &grecord)
                }
            };

            let gandi_api_response = match gandi_network_response {
                Ok(result) => result,
                Err(e) => {
                    println!("Network error: Unable to connect to gandi's API");
                    println!("{}", e);
                    continue;
                }
            };

            if record_state == takina::State::DiffRecord {
                match gandi_api_response.status() {
                    StatusCode::CREATED => continue,
                    StatusCode::FORBIDDEN => {
                        println!("API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it.");
                        println!("{}", gandi_api_response.text().unwrap());
                        continue;
                    }
                    StatusCode::UNAUTHORIZED => {
                        println!("API HTTP Error: UNAUTHORIZED Bad authentication attempt because of a wrong API Key.");
                        println!("{}", gandi_api_response.text().unwrap());
                        break;
                    }
                    _ => {
                        println!("API HTTP Error: UNEXPECTED status code see below");
                        println!("{}", gandi_api_response.text().unwrap());
                        continue;
                    }
                };
            } else if record_state == takina::State::CreateRecord {
                match gandi_api_response.status() {
                    StatusCode::OK => {
                        println!("API HTTP Error: OK Record Already Exsist");
                        println!("This is unexpected since takina must handle this case by itself");
                        println!("{}", gandi_api_response.text().unwrap());
                        continue;
                    }
                    StatusCode::CREATED => {
                        println!(
                            "Info: New Record Created, {}.{} IpAddr: {} TTL: {}",
                            record.name(),
                            domain.name(),
                            addr,
                            record.ttl(),
                        );
                        continue;
                    }
                    StatusCode::FORBIDDEN => {
                        println!("API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it.");
                        println!("{}", gandi_api_response.text().unwrap());
                        continue;
                    }
                    StatusCode::CONFLICT => {
                        println!("API HTTP Error: CONFLICT A record with that name / type pair already exists");
                        println!("{}", gandi_api_response.text().unwrap());
                        continue;
                    }
                    StatusCode::UNAUTHORIZED => continue,
                    _ => {
                        println!("API HTTP Error: Unexpected status code see below");
                        println!("{}", gandi_api_response.text().unwrap());
                        continue;
                    }
                };
            }
        }
    }
}

fn read_config(path: &str) -> Result<String, io::Error> {
    std::fs::read_to_string(path)
}

fn parse_config(conf: &str) -> takina::Config {
    match toml::from_str(conf) {
        Ok(conf) => conf,
        Err(e) => panic!("Failed to parse toml configuration file {e}"),
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
        .header("Authorization", "Apikey ".to_owned() + domain.api_key())
        .send()?;
    Ok(res)
}

fn update_record(
    domain: &takina::Domain,
    record: &takina::Record,
    grecord: &takina::GandiRecord,
) -> Result<Response, Error> {
    let endpoint = format!(
        "https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        domain.name(),
        record.name(),
        record.rtype()
    );
    let client = reqwest::blocking::Client::new();
    let res = client
        .put(&endpoint)
        .header("Authorization", "Apikey ".to_owned() + domain.api_key())
        .body(serde_json::to_string(&grecord).unwrap())
        .send()?;
    Ok(res)
}

fn create_record(
    domain: &takina::Domain,
    record: &takina::Record,
    grecord: &takina::GandiRecord,
) -> Result<Response, Error> {
    let endpoint = format!(
        "https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        domain.name(),
        record.name(),
        record.rtype()
    );
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(&endpoint)
        .header("Authorization", "Apikey ".to_owned() + domain.api_key())
        .body(serde_json::to_string(&grecord).unwrap())
        .send()?;
    Ok(res)
}
