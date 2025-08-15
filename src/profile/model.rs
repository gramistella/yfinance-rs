#[derive(Debug, Clone, PartialEq)]
pub struct Address {
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Company {
    pub name: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub website: Option<String>,
    pub address: Option<Address>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fund {
    pub name: String,
    pub family: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Profile {
    Company(Company),
    Fund(Fund),
}