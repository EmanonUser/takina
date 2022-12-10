use clap::Parser;
use std::process::ExitCode;
use ureq::Error;
use log::{info, warn, error};
use env_logger::Env;
use args::TakinaArgs;
use takina::{create_record, get_ipv4, get_ipv6, get_record, update_record};
use takina::{ApiRecord, Record, TakinaState};

mod args;

fn main() -> ExitCode {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = TakinaArgs::parse();

    let config_path = match args.config {
        Some(s) => s,
        None => "./takina.toml".to_string(),
    };

    if args.check {
        let read_config_result = match std::fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) => {
                error!("IO Error: Failed to read configuration file {e}");
                return ExitCode::FAILURE
            }
        };

        let domains: takina::Config = match toml::from_str(&read_config_result) {
            Ok(conf) => conf,
            Err(e) => {
                error!("Configuration Error: Failed to parse toml configuration file {e}");
                return ExitCode::FAILURE;
            }
        };

        for domain in &domains.domain {
            domain.validate_fields();
            for record in domain.record() {
                record.validate_fields();
            }
        }
        info!("{config_path} configuration file is valid");
        return ExitCode::SUCCESS;
    }

    let read_config_result = match std::fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => {
            error!("IO Error: Failed to read configuration file {e}");
            return ExitCode::FAILURE
        }
    };

    let domains: takina::Config = match toml::from_str(&read_config_result) {
        Ok(conf) => conf,
        Err(e) => {
            error!("Configuration Error: Failed to parse toml configuration file {e}");
            return ExitCode::FAILURE;
        }
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
                warn!(
                    "Skiping Record {}.{} Type: {}",
                    record.name(),
                    domain.name(),
                    record.rtype(),
                );
                continue;
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
                        error!(
                        "API HTTP Error: Bad authentication attempt because of a wrong API Key {e}");
                        break;
                    }
                    Error::Status(403, _) => {
                        error!(
                        "API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it {e}");
                        break;
                    }
                    Error::Status(code, res) => {
                        error!("API HTTP Error: UNEXPECTED status code: {code}");
                        error!("{}", res.into_string().unwrap());
                        continue;
                    }
                    Error::Transport(t) => {
                        error!("Transport Error: {t}");
                        continue;
                    }
                },
            };

            let record_state = match res.status() {
                200 => TakinaState::DiffRecord,
                404 => TakinaState::CreateRecord,
                _ => {
                    error!("API HTTP Error: UNEXPECTED status code: {}", res.into_string().unwrap());
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
                        info!(
                            "No update Needed for {}.{} IpAddr: {} TTL: {}",
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
                                    info!(
                                        "Record already exist {}.{} IpAddr: {} TTL: {}",
                                        record.name(),
                                        domain.name(),
                                        addr,
                                        record.ttl(),
                                    );
                                }
                                201 => {
                                    info!(
                                        "Record created, {}.{} IpAddr: {} TTL: {}",
                                        record.name(),
                                        domain.name(),
                                        addr,
                                        record.ttl(),
                                    );
                                }

                                code => error!("Ureq Error: Unexpected response code: {code}"),
                            };
                        }
                        Err(e) => match *e {
                            Error::Status(401, r) => {
                                error!("API HTTP Error: UNAUTHORIZED Bad authentication attempt because of a wrong API Key");
                                error!("{}", r.into_string().unwrap());
                                break;
                            }
                            Error::Status(403, r) => {
                                error!("API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it");
                                error!("{}", r.into_string().unwrap());
                                break;
                            }
                            Error::Status(409, r) => {
                                error!("API HTTP Error: CONFLICT A record with that name / type pair already exists");
                                error!("{}", r.into_string().unwrap());
                                continue;
                            }
                            Error::Status(code, r) => {
                                error!("API HTTP Error: UNEXPECTED status code: {code}");
                                error!("{}", r.into_string().unwrap());
                                continue;
                            }
                            Error::Transport(t) => {
                                error!("Transport Error: {t}");
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
                                    info!(
                                        "Record updated, {}.{} IpAddr: {} TTL: {}",
                                        record.name(),
                                        domain.name(),
                                        addr,
                                        record.ttl()
                                    );
                                }
                                code => error!("Ureq Error: Unexpected response code: {code}"),
                            };
                        }
                        Err(e) => match *e {
                            Error::Status(401, r) => {
                                error!("API HTTP Error: UNAUTHORIZED Bad authentication attempt because of a wrong API Key");
                                error!("{}", r.into_string().unwrap());
                                break;
                            }
                            Error::Status(403, r) => {
                                error!("API HTTP Error: FORBIDDEN Access to the resource is denied. Mainly due to a lack of permissions to access it");
                                error!("{}", r.into_string().unwrap());
                                break;
                            }
                            Error::Status(code, r) => {
                                error!("API HTTP Error: UNEXPECTED status code: {code}");
                                error!("{}", r.into_string().unwrap());
                                continue;
                            }
                            Error::Transport(t) => {
                                error!("Transport Error: {t}");
                                continue;
                            }
                        },
                    };
                }
            }
        }
    }
    ExitCode::SUCCESS
}
