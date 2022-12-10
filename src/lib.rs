use serde::{Deserialize, Serialize};
use ureq::{Error, Response};
use log::{warn, error};
#[derive(Deserialize, Debug)]
pub struct Config {
    pub domain: Vec<Domain>,
}

impl Config {
    pub fn domain(&self) -> &[Domain] {
        self.domain.as_ref()
    }
}

#[derive(Deserialize, Debug)]
pub struct Domain {
    name: String,
    api_key: String,
    pub record: Vec<Record>,
}

impl Domain {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn api_key(&self) -> &str {
        self.api_key.as_ref()
    }

    pub fn record(&self) -> &[Record] {
        self.record.as_ref()
    }

    pub fn validate_fields(&self) {
        for c in self.name.chars() {
            if !c.is_ascii_alphanumeric() && c != '.' {
                error!(
                    "Configuration Error: Domain name does not match ascii_alphanumeric pattern"
                );
            }
        }

        if self.api_key.len() < 3 {
            error!("Configuration Error: api Key seems empty");
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Record {
    #[serde(rename(deserialize = "name"))]
    rrset_name: String,
    #[serde(rename(deserialize = "type"))]
    rrset_type: String,
    #[serde(skip_deserializing)]
    rrset_values: Vec<String>,
    #[serde(rename(deserialize = "ttl"))]
    rrset_ttl: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiRecord {
    rrset_name: String,
    rrset_type: String,
    rrset_values: Vec<String>,
    rrset_ttl: u32,
    rrset_href: String,
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.rrset_values == other.rrset_values && self.rrset_ttl == other.rrset_ttl
    }
}

impl Record {
    pub fn validate_fields(&self) {
        for c in self.rrset_name.chars() {
            if !c.is_ascii_alphanumeric() && c != '@' {
                error!(
                    "Configuration Error: Record name does not match ascii_alphanumeric pattern"
                );
            }
        }

        if self.rrset_type != "AAAA" && self.rrset_type != "A" {
            error!("Configuration Error: Record type is neither A or AAAA")
        }

        if self.rrset_ttl > 2_592_000 || self.rrset_ttl < 300 {
            error!("Configuration Error: TTL size exceed gandis's minimum or maximum value (2_592_000)");
        }
    }

    pub fn name(&self) -> &str {
        self.rrset_name.as_ref()
    }

    pub fn rtype(&self) -> &str {
        self.rrset_type.as_ref()
    }

    pub fn set_rrset_values(&mut self, rrset_values: Vec<String>) {
        self.rrset_values = rrset_values;
    }

    pub fn ttl(&self) -> u32 {
        self.rrset_ttl
    }

    pub fn from_api_record(apirecord: ApiRecord) -> Self {
        Record {
            rrset_name: apirecord.rrset_name,
            rrset_type: apirecord.rrset_type,
            rrset_values: apirecord.rrset_values,
            rrset_ttl: apirecord.rrset_ttl,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum TakinaState {
    CreateRecord,
    DiffRecord,
}

pub fn get_record(domain: &Domain, record: &Record) -> Result<Response, Box<Error>> {
    let endpoint = format!(
        "https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        domain.name(),
        record.name(),
        record.rtype()
    );
    let key = format!("{} {}", "ApiKey", domain.api_key());
    let res = ureq::get(&endpoint).set("Authorization", &key).call()?;
    Ok(res)
}

pub fn update_record(domain: &Domain, record: &Record) -> Result<Response, Box<Error>> {
    let endpoint = format!(
        "https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        domain.name(),
        record.name(),
        record.rtype()
    );
    let key = format!("{} {}", "ApiKey", domain.api_key());
    let res = ureq::put(&endpoint)
        .set("Authorization", &key)
        .send_string(&serde_json::to_string(&record).unwrap())?;
    Ok(res)
}

pub fn create_record(domain: &Domain, record: &Record) -> Result<Response, Box<Error>> {
    let endpoint = format!(
        "https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        domain.name(),
        record.name(),
        record.rtype()
    );
    let key = format!("{} {}", "ApiKey", domain.api_key());
    let res = ureq::post(&endpoint)
        .set("Authorization", &key)
        .send_string(&serde_json::to_string(&record).unwrap())?;
    Ok(res)
}

pub fn get_ipv4() -> Option<Response> {
    let endpoint = "https://api4.ipify.org?format=txt";
    let res = ureq::get(endpoint).call();

    match res {
        Ok(r) => Some(r),
        Err(e) => {
            warn!("Network Error: Failed to fetch IPv4 addr {e}");
            None
        }
    }
}

pub fn get_ipv6() -> Option<Response> {
    let endpoint = "https://api6.ipify.org?format=txt";
    let res = ureq::get(endpoint).call();

    match res {
        Ok(r) => Some(r),
        Err(e) => {
            warn!("Network Error: Failed to fetch IPv6 addr {e}");
            None
        }
    }
}
