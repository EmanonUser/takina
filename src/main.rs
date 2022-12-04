use clap::Parser;
use std::process::ExitCode;
use ureq::Error;

use args::TakinaArgs;
use takina::{create_record, get_ipv4, get_ipv6, get_record, update_record};
use takina::{ApiRecord, Record, TakinaState};

mod args;

fn main() -> ExitCode {
    let args = TakinaArgs::parse();

    let config_path = match args.config {
        Some(s) => s,
        None => "./takina.toml".to_string(),
    };

    if args.check {
        let read_config_result = match std::fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) => panic!("IO Error: Failed to read configuration file {e}"),
        };

        let domains: takina::Config = match toml::from_str(&read_config_result) {
            Ok(conf) => conf,
            Err(e) => panic!("Failed to parse toml configuration file {e}"),
        };

        for domain in &domains.domain {
            domain.validate_fields();
            for record in domain.record() {
                record.validate_fields();
            }
        }
        println!("{config_path} configuration file is valid");
        return ExitCode::SUCCESS;
    }

    let read_config_result = match std::fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => panic!("IO Error: Failed to read configuration file {e}"),
    };

    let domains: takina::Config = match toml::from_str(&read_config_result) {
        Ok(conf) => conf,
        Err(e) => panic!("Failed to parse toml configuration file {e}"),
    };

    for domain in domains.domain() {
        domain.validate_fields();
        for record in domain.record() {
            record.validate_fields()
        }
    }

    let mut disable_ipv4 = true;
    let mut disable_ipv6 = true;

    for domain in domains.domain() {
        for record in domain.record() {
            if record.rtype() == "A" {
                disable_ipv4 = false;
            } else if record.rtype() == "AAAA" {
                disable_ipv6 = false;
            }
        }
    }

    let ipv4 = match get_ipv4() {
        Some(res) => res
            .into_string()
            .expect("Error: Failed to parse ipify response to string"),
        None => {
            disable_ipv4 = true;
            String::from("")
        }
    };

    let ipv6 = match get_ipv6() {
        Some(res) => res
            .into_string()
            .expect("Error: Failed to parse ipify response to string"),
        None => {
            disable_ipv6 = true;
            String::from("")
        }
    };

    for domain in domains.domain() {
        for record in domain.record() {
            let mut addr = String::default();
            if record.rtype() == "A" && disable_ipv4 || record.rtype() == "AAAA" && disable_ipv6 {
                println!(
                    "Info: Skiping Record {}.{} Type: {}",
                    record.name(),
                    domain.name(),
                    record.rtype(),
                );
            } else if record.rtype() == "A" && !disable_ipv4 {
                addr = ipv4.to_owned();
            } else if record.rtype() == "AAAA" && !disable_ipv6 {
                addr = ipv6.to_owned();
            }

            let res = match get_record(domain, record) {
                Ok(r) => r,
                Err(e) => match *e {
                    Error::Status(404, r) => r,
                    Error::Status(401, _) => {
                        println!(
                        "API HTTP Error: Bad authentication attempt because of a wrong API Key.");
                        println!("{}", e);
                        break;
                    }
                    Error::Status(403, _) => {
                        println!(
                        "API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it.");
                        println!("{}", e);
                        break;
                    }
                    Error::Status(code, res) => {
                        println!("API HTTP Error: UNEXPECTED status code see below");
                        println!("{}", code);
                        println!("{}", res.into_string().unwrap());
                        continue;
                    }
                    Error::Transport(t) => {
                        println!("Transport Error: see status code see below");
                        println!("{}", t);
                        continue;
                    }
                },
            };

            let record_state = match res.status() {
                200 => TakinaState::DiffRecord,
                404 => TakinaState::CreateRecord,
                _ => {
                    println!("API HTTP Error: UNEXPECTED status code see below");
                    println!("{}", res.into_string().unwrap());
                    continue;
                }
            };

            let res = match record_state {
                TakinaState::CreateRecord => {
                    let mut config_record = record.clone();
                    config_record.set_rrset_values(vec![addr.clone()]);
                    create_record(domain, &config_record)
                }

                TakinaState::DiffRecord => {
                    let mut config_record = record.clone();
                    config_record.set_rrset_values(vec![addr.clone()]);

                    let response = res
                        .into_string()
                        .expect("Ureq Error: Failed to convert response to string");
                    let api_record: ApiRecord = serde_json::from_str(&response)
                        .expect("Serde Error: Failed to deserialize");
                    let api_record: Record = Record::from_api_record(api_record);

                    if config_record != api_record {
                        update_record(domain, &config_record)
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
            };

            match record_state {
                TakinaState::CreateRecord => {
                    match res {
                        Ok(r) => {
                            match r.status() {
                                200 => {
                                    println!(
                                        "Info: Record already exist {}.{} IpAddr: {} TTL: {}",
                                        record.name(),
                                        domain.name(),
                                        addr,
                                        record.ttl(),
                                    );
                                }
                                201 => {
                                    println!(
                                        "Info: Record created, {}.{} IpAddr: {} TTL: {}",
                                        record.name(),
                                        domain.name(),
                                        addr,
                                        record.ttl(),
                                    );
                                }

                                code => println!("Ureq Error: Unexpected response code: {code}"),
                            };
                        }
                        Err(e) => match *e {
                            Error::Status(401, r) => {
                                println!("API HTTP Error: UNAUTHORIZED Bad authentication attempt because of a wrong API Key.");
                                println!("{}", r.into_string().unwrap());
                                break;
                            }
                            Error::Status(403, r) => {
                                println!("API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it.");
                                println!("{}", r.into_string().unwrap());
                            }
                            Error::Status(409, r) => {
                                println!("API HTTP Error: CONFLICT A record with that name / type pair already exists");
                                println!("{}", r.into_string().unwrap());
                                continue;
                            }
                            Error::Status(code, r) => {
                                println!("API HTTP Error: UNEXPECTED status code see below");
                                println!("{}", code);
                                println!("{}", r.into_string().unwrap());
                                continue;
                            }
                            Error::Transport(t) => {
                                println!("Transport Error: see below");
                                println!("{}", t);
                                continue;
                            }
                        },
                    };
                }
                TakinaState::DiffRecord => {
                    match res {
                        Ok(r) => {
                            match r.status() {
                                201 => {
                                    println!(
                                        "Info: Record Updated, {}.{} IpAddr: {} TTL: {}",
                                        record.name(),
                                        domain.name(),
                                        addr,
                                        record.ttl()
                                    );
                                }
                                code => println!("Ureq Error: Unexpected response code: {code}"),
                            };
                        }
                        Err(e) => match *e {
                            Error::Status(401, r) => {
                                println!("API HTTP Error: UNAUTHORIZED Bad authentication attempt because of a wrong API Key.");
                                println!("{}", r.into_string().unwrap());
                            }
                            Error::Status(403, r) => {
                                println!("API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it.");
                                println!("{}", r.into_string().unwrap());
                            }
                            Error::Status(code, r) => {
                                println!("API HTTP Error: UNEXPECTED status code see below");
                                println!("{}", code);
                                println!("{}", r.into_string().unwrap());
                            }
                            Error::Transport(t) => {
                                println!("Transport Error: see below");
                                println!("{}", t);
                            }
                        },
                    };
                }
            }
        }
    }
    ExitCode::SUCCESS
}
