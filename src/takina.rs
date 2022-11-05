use serde::{Serialize,Deserialize};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub domain: Vec<Domain>

}
#[derive(Deserialize, Debug)]
pub struct Domain {
    name: String,
    api_key: String,
    record: Vec<Record>
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
    ttl: u32
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

pub enum State {
    CreateRecord,
    DiffRecord,
}

impl GandiRecord {
    pub fn diff(&self, record: &Record, values: Vec<String>) -> bool {
        if self.rrset_ttl == record.ttl && self.rrset_values == values {
            false
        } else  {
            true
        }
    }
}

impl Record {
    pub fn validate_fields(&self) {

        for c in self.name.chars() {
            if !c.is_ascii_alphanumeric() {
                panic!("Record name does not match ascii_alphanumeric pattern")
            }
        }

        let rrset_type = self.rtype.to_uppercase();
        if rrset_type != "AAAA" && rrset_type != "A" {
            panic!("Record type is neither A or AAAA")
        }

        if self.ttl > 2_592_000 {
            panic!("TTL size exceed gandis's maximum value (2_592_000)")
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