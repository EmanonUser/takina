use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub domain: Vec<Domain>,
}
#[derive(Deserialize, Debug)]
pub struct Domain {
    name: String,
    api_key: String,
    record: Vec<Record>,
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
}

#[derive(Deserialize, Debug)]
pub struct Record {
    name: String,
    #[serde(rename = "type")]
    rtype: String,
    ttl: u32,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GandiRecord {
    #[serde(skip_serializing)]
    pub rrset_name: String,
    #[serde(skip_serializing)]
    pub rrset_type: String,
    pub rrset_values: Vec<String>,
    pub rrset_ttl: u32,
}

#[derive(PartialEq, Eq)]
pub enum State {
    CreateRecord,
    DiffRecord,
}

impl GandiRecord {
    pub fn diff(&self, record: &Record, values: Vec<String>) -> bool {
        !(self.rrset_ttl == record.ttl && self.rrset_values == values)
    }
}

impl Record {
    pub fn validate_fields(&self) {
        for c in self.name.chars() {
            if !c.is_ascii_alphanumeric() {
                panic!(
                    "Configuration Error: Record name does not match ascii_alphanumeric pattern"
                );
            }
        }

        if self.rtype != "AAAA" && self.rtype != "A" {
            panic!("Configuration Error: Record type is neither A or AAAA")
        }

        if self.ttl > 2_592_000 || self.ttl < 300 {
            panic!("Configuration Error: TTL size exceed gandis's minimum or maximum value (2_592_000)");
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn rtype(&self) -> &str {
        self.rtype.as_ref()
    }

    pub fn ttl(&self) -> u32 {
        self.ttl
    }
}
