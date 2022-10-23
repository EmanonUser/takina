use serde::{Deserialize, Serialize};

pub const DOMAIN_NAME: &str = "emanon.moe";


#[derive(Serialize, Deserialize, Debug)]
pub struct Record {
    #[serde(skip_serializing)]
    rrset_name: String,
    #[serde(skip_serializing)]
    rrset_type: String,
    rrset_values: Vec<String>,
    rrset_ttl: u32,
}

impl Record {
    pub fn new(rrset_name: String, rrset_type: String, 
        rrset_ttl: u32, rrset_values: Vec<String>) -> Self {

            for c in rrset_name.chars() {
                if !c.is_ascii_alphanumeric() {
                    panic!("Record name does not match ascii_alphanumeric pattern")
                }
            }

            let rrset_type = rrset_type.to_uppercase();

            if rrset_type != "AAAA" && rrset_type != "A" {
                panic!("Record type is neither A or AAAA") 
            }

            if rrset_ttl > 2_592_000 {
                panic!("TTL size exceed gandis's maximum value (2_592_000)")
            }

            Record {
                rrset_name,
                rrset_type,
                rrset_ttl,
                rrset_values,
            }  
        }
    
    pub fn diff(&self, record: &Self) -> RecordState {
        if self.rrset_values ==  record.rrset_values  {
            if self.rrset_ttl == record.rrset_ttl {
                RecordState::Same
            }
            else {
                RecordState::Diff
            }
        }
        else {
            RecordState::Diff
        }
    }
    
    pub fn get_name(&self) -> &str {
        &self.rrset_name
    }

    pub fn get_type(&self) -> &str {
        &self.rrset_type
    }

    pub fn get_values(&self) -> &Vec<String> {
        &self.rrset_values
    }

    pub fn get_ttl(&self) -> u32 {
        self.rrset_ttl
    }
}

// impl Default for Record {
//     fn default() -> Self {
//         Record { 
//             rrset_name: String::from(""), 
//             rrset_type: String::from(""), 
//             rrset_ttl: 300, 
//             rrset_values: vec![String::from("")],
//         }
//     }
// }

#[derive(Debug)]
pub enum RecordState {
    Same,
    Diff,
}