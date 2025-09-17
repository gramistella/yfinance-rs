//! Helpers for inferring currencies from country information.

use std::{collections::HashMap, sync::LazyLock};

use paft::prelude::Currency;

/// Normalized country → currency code pairs.
///
/// Keys must be uppercase and ASCII; values are ISO 4217 currency codes.
const COUNTRY_TO_CURRENCY_RAW: &[(&str, &str)] = &[
    ("UNITED STATES", "USD"),
    ("UNITED STATES OF AMERICA", "USD"),
    ("US", "USD"),
    ("USA", "USD"),
    ("CANADA", "CAD"),
    ("MEXICO", "MXN"),
    ("BRAZIL", "BRL"),
    ("ARGENTINA", "ARS"),
    ("CHILE", "CLP"),
    ("COLOMBIA", "COP"),
    ("PERU", "PEN"),
    ("URUGUAY", "UYU"),
    ("PARAGUAY", "PYG"),
    ("BOLIVIA", "BOB"),
    ("ECUADOR", "USD"),
    ("VENEZUELA", "VES"),
    ("COSTA RICA", "CRC"),
    ("GUATEMALA", "GTQ"),
    ("HONDURAS", "HNL"),
    ("NICARAGUA", "NIO"),
    ("PANAMA", "USD"),
    ("EL SALVADOR", "USD"),
    ("BELIZE", "BZD"),
    ("DOMINICAN REPUBLIC", "DOP"),
    ("JAMAICA", "JMD"),
    ("TRINIDAD AND TOBAGO", "TTD"),
    ("BARBADOS", "BBD"),
    ("BAHAMAS", "BSD"),
    ("BERMUDA", "BMD"),
    ("CAYMAN ISLANDS", "KYD"),
    ("ARUBA", "AWG"),
    ("CURACAO", "ANG"),
    ("BRITISH VIRGIN ISLANDS", "USD"),
    ("PUERTO RICO", "USD"),
    ("UNITED KINGDOM", "GBP"),
    ("ENGLAND", "GBP"),
    ("SCOTLAND", "GBP"),
    ("WALES", "GBP"),
    ("NORTHERN IRELAND", "GBP"),
    ("IRELAND", "EUR"),
    ("FRANCE", "EUR"),
    ("GERMANY", "EUR"),
    ("ITALY", "EUR"),
    ("SPAIN", "EUR"),
    ("PORTUGAL", "EUR"),
    ("NETHERLANDS", "EUR"),
    ("BELGIUM", "EUR"),
    ("LUXEMBOURG", "EUR"),
    ("AUSTRIA", "EUR"),
    ("SWITZERLAND", "CHF"),
    ("SWEDEN", "SEK"),
    ("NORWAY", "NOK"),
    ("DENMARK", "DKK"),
    ("FINLAND", "EUR"),
    ("ICELAND", "ISK"),
    ("POLAND", "PLN"),
    ("CZECH REPUBLIC", "CZK"),
    ("CZECHIA", "CZK"),
    ("HUNGARY", "HUF"),
    ("SLOVAKIA", "EUR"),
    ("SLOVENIA", "EUR"),
    ("CROATIA", "EUR"),
    ("ROMANIA", "RON"),
    ("BULGARIA", "BGN"),
    ("GREECE", "EUR"),
    ("CYPRUS", "EUR"),
    ("MALTA", "EUR"),
    ("ESTONIA", "EUR"),
    ("LATVIA", "EUR"),
    ("LITHUANIA", "EUR"),
    ("UKRAINE", "UAH"),
    ("BELARUS", "BYN"),
    ("RUSSIA", "RUB"),
    ("TURKEY", "TRY"),
    ("SERBIA", "RSD"),
    ("BOSNIA AND HERZEGOVINA", "BAM"),
    ("NORTH MACEDONIA", "MKD"),
    ("ALBANIA", "ALL"),
    ("MONTENEGRO", "EUR"),
    ("KOSOVO", "EUR"),
    ("ARMENIA", "AMD"),
    ("GEORGIA", "GEL"),
    ("AZERBAIJAN", "AZN"),
    ("KAZAKHSTAN", "KZT"),
    ("UZBEKISTAN", "UZS"),
    ("TURKMENISTAN", "TMT"),
    ("KYRGYZSTAN", "KGS"),
    ("TAJIKISTAN", "TJS"),
    ("CHINA", "CNY"),
    ("PEOPLES REPUBLIC OF CHINA", "CNY"),
    ("HONG KONG", "HKD"),
    ("MACAU", "MOP"),
    ("TAIWAN", "TWD"),
    ("JAPAN", "JPY"),
    ("SOUTH KOREA", "KRW"),
    ("REPUBLIC OF KOREA", "KRW"),
    ("NORTH KOREA", "KPW"),
    ("INDIA", "INR"),
    ("PAKISTAN", "PKR"),
    ("BANGLADESH", "BDT"),
    ("SRI LANKA", "LKR"),
    ("NEPAL", "NPR"),
    ("BHUTAN", "BTN"),
    ("MALDIVES", "MVR"),
    ("MYANMAR", "MMK"),
    ("THAILAND", "THB"),
    ("VIETNAM", "VND"),
    ("LAOS", "LAK"),
    ("CAMBODIA", "KHR"),
    ("MALAYSIA", "MYR"),
    ("SINGAPORE", "SGD"),
    ("INDONESIA", "IDR"),
    ("PHILIPPINES", "PHP"),
    ("BRUNEI", "BND"),
    ("MONGOLIA", "MNT"),
    ("AUSTRALIA", "AUD"),
    ("NEW ZEALAND", "NZD"),
    ("FIJI", "FJD"),
    ("PAPUA NEW GUINEA", "PGK"),
    ("NEW CALEDONIA", "XPF"),
    ("FRENCH POLYNESIA", "XPF"),
    ("SAMOA", "WST"),
    ("TONGA", "TOP"),
    ("VANUATU", "VUV"),
    ("SOLOMON ISLANDS", "SBD"),
    ("EAST TIMOR", "USD"),
    ("TIMOR-LESTE", "USD"),
    ("UNITED ARAB EMIRATES", "AED"),
    ("SAUDI ARABIA", "SAR"),
    ("QATAR", "QAR"),
    ("KUWAIT", "KWD"),
    ("BAHRAIN", "BHD"),
    ("OMAN", "OMR"),
    ("JORDAN", "JOD"),
    ("LEBANON", "LBP"),
    ("ISRAEL", "ILS"),
    ("PALESTINE", "ILS"),
    ("IRAQ", "IQD"),
    ("IRAN", "IRR"),
    ("AFGHANISTAN", "AFN"),
    ("SYRIA", "SYP"),
    ("YEMEN", "YER"),
    ("EGYPT", "EGP"),
    ("MOROCCO", "MAD"),
    ("ALGERIA", "DZD"),
    ("TUNISIA", "TND"),
    ("LIBYA", "LYD"),
    ("SUDAN", "SDG"),
    ("SOUTH SUDAN", "SSP"),
    ("NIGERIA", "NGN"),
    ("GHANA", "GHS"),
    ("COTE DIVOIRE", "XOF"),
    ("COTE D IVOIRE", "XOF"),
    ("COTE D'IVOIRE", "XOF"),
    ("SENEGAL", "XOF"),
    ("MALI", "XOF"),
    ("BENIN", "XOF"),
    ("BURKINA FASO", "XOF"),
    ("NIGER", "XOF"),
    ("TOGO", "XOF"),
    ("GUINEA-BISSAU", "XOF"),
    ("GUINEA BISSAU", "XOF"),
    ("CAMEROON", "XAF"),
    ("CHAD", "XAF"),
    ("CENTRAL AFRICAN REPUBLIC", "XAF"),
    ("REPUBLIC OF THE CONGO", "XAF"),
    ("CONGO", "XAF"),
    ("GABON", "XAF"),
    ("EQUATORIAL GUINEA", "XAF"),
    ("GAMBIA", "GMD"),
    ("GUINEA", "GNF"),
    ("SIERRA LEONE", "SLE"),
    ("LIBERIA", "LRD"),
    ("ETHIOPIA", "ETB"),
    ("ERITREA", "ERN"),
    ("DJIBOUTI", "DJF"),
    ("KENYA", "KES"),
    ("UGANDA", "UGX"),
    ("TANZANIA", "TZS"),
    ("RWANDA", "RWF"),
    ("BURUNDI", "BIF"),
    ("SOMALIA", "SOS"),
    ("SEYCHELLES", "SCR"),
    ("MADAGASCAR", "MGA"),
    ("MAURITIUS", "MUR"),
    ("MOZAMBIQUE", "MZN"),
    ("ZIMBABWE", "ZWL"),
    ("ZAMBIA", "ZMW"),
    ("MALAWI", "MWK"),
    ("ANGOLA", "AOA"),
    ("NAMIBIA", "NAD"),
    ("BOTSWANA", "BWP"),
    ("SOUTH AFRICA", "ZAR"),
    ("LESOTHO", "LSL"),
    ("ESWATINI", "SZL"),
    ("SWAZILAND", "SZL"),
    ("COMOROS", "KMF"),
    ("MAURITANIA", "MRU"),
    ("SAO TOME AND PRINCIPE", "STN"),
    ("GRENADA", "XCD"),
    ("SAINT LUCIA", "XCD"),
    ("SAINT VINCENT AND THE GRENADINES", "XCD"),
    ("ANTIGUA AND BARBUDA", "XCD"),
    ("DOMINICA", "XCD"),
    ("SAINT KITTS AND NEVIS", "XCD"),
];

/// Precomputed lookup table using `COUNTRY_TO_CURRENCY_RAW`.
static COUNTRY_TO_CURRENCY: LazyLock<HashMap<&'static str, Currency>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for (country, code) in COUNTRY_TO_CURRENCY_RAW {
        map.insert(*country, Currency::from((*code).to_string()));
    }
    map
});

/// Normalize a country string to an uppercase ASCII key.
fn normalize_country(country: &str) -> Option<String> {
    let trimmed = country.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut buf = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            'A'..='Z' | '0'..='9' => buf.push(ch),
            'a'..='z' => buf.push(ch.to_ascii_uppercase()),
            ' ' | '\t' | '\n' | '\r' => buf.push(' '),
            '-' | '_' | '/' | ',' | '.' | ';' | ':' | '&' | '(' | ')' | '[' | ']' | '{' | '}' => {
                buf.push(' ')
            }
            '\'' | '`' | '"' => buf.push(' '),
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => {
                buf.push('A')
            }
            'ç' | 'Ç' => buf.push('C'),
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => buf.push('E'),
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => buf.push('I'),
            'ñ' | 'Ñ' => buf.push('N'),
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' => buf.push('O'),
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => buf.push('U'),
            'ý' | 'ÿ' | 'Ý' => buf.push('Y'),
            _ => {
                // Ignore other symbols to keep normalization simple.
            }
        }
    }

    let normalized = buf
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Attempt to infer a currency from a country string.
///
/// Returns `None` if the country string is empty or cannot be matched.
pub(crate) fn currency_for_country(country: &str) -> Option<Currency> {
    let normalized = normalize_country(country)?;

    if let Some(currency) = COUNTRY_TO_CURRENCY.get(normalized.as_str()) {
        return Some(currency.clone());
    }

    heuristic_currency_match(&normalized)
}

fn heuristic_currency_match(normalized: &str) -> Option<Currency> {
    let contains = |needle: &str| normalized.contains(needle);

    if contains("UNITED STATES") {
        return Some(Currency::USD);
    }
    if contains("UNITED KINGDOM") || contains("ENGLAND") || contains("SCOTLAND") {
        return Some(Currency::GBP);
    }
    if contains("EUROPEAN UNION") || contains("EURO AREA") {
        return Some(Currency::EUR);
    }
    if contains("HONG KONG") {
        return Some(Currency::HKD);
    }
    if contains("MACAU") {
        return Some(Currency::from("MOP".to_string()));
    }
    if contains("TAIWAN") {
        return Some(Currency::from("TWD".to_string()));
    }
    if contains("KOREA") {
        return Some(Currency::KRW);
    }
    if contains("AUSTRALIA") {
        return Some(Currency::AUD);
    }
    if contains("NEW ZEALAND") {
        return Some(Currency::NZD);
    }
    if contains("SINGAPORE") {
        return Some(Currency::SGD);
    }
    if contains("MALAYSIA") {
        return Some(Currency::MYR);
    }
    if contains("INDONESIA") {
        return Some(Currency::IDR);
    }
    if contains("PHILIPPINES") {
        return Some(Currency::PHP);
    }
    if contains("VIETNAM") {
        return Some(Currency::VND);
    }
    if contains("THAILAND") {
        return Some(Currency::THB);
    }
    if contains("INDIA") {
        return Some(Currency::INR);
    }
    if contains("SWITZERLAND") {
        return Some(Currency::CHF);
    }
    if contains("NORWAY") {
        return Some(Currency::NOK);
    }
    if contains("SWEDEN") {
        return Some(Currency::SEK);
    }
    if contains("DENMARK") {
        return Some(Currency::DKK);
    }
    if contains("ICELAND") {
        return Some(Currency::from("ISK".to_string()));
    }
    if contains("JAPAN") {
        return Some(Currency::JPY);
    }
    if contains("CHINA") {
        return Some(Currency::CNY);
    }
    if contains("BRAZIL") {
        return Some(Currency::BRL);
    }
    if contains("MEXICO") {
        return Some(Currency::MXN);
    }
    if contains("CANADA") {
        return Some(Currency::CAD);
    }
    if contains("RUSSIA") {
        return Some(Currency::RUB);
    }
    if contains("TURKEY") {
        return Some(Currency::TRY);
    }
    if contains("SOUTH AFRICA") {
        return Some(Currency::ZAR);
    }
    if contains("ISRAEL") {
        return Some(Currency::ILS);
    }
    if contains("SAUDI ARABIA") {
        return Some(Currency::from("SAR".to_string()));
    }
    if contains("UNITED ARAB EMIRATES") {
        return Some(Currency::from("AED".to_string()));
    }
    if contains("QATAR") {
        return Some(Currency::from("QAR".to_string()));
    }
    if contains("KUWAIT") {
        return Some(Currency::from("KWD".to_string()));
    }
    if contains("BAHRAIN") {
        return Some(Currency::from("BHD".to_string()));
    }
    if contains("OMAN") {
        return Some(Currency::from("OMR".to_string()));
    }
    if contains("EGYPT") {
        return Some(Currency::from("EGP".to_string()));
    }
    if contains("NIGERIA") {
        return Some(Currency::from("NGN".to_string()));
    }
    if contains("GHANA") {
        return Some(Currency::from("GHS".to_string()));
    }
    if contains("KENYA") {
        return Some(Currency::from("KES".to_string()));
    }
    if contains("MOROCCO") {
        return Some(Currency::from("MAD".to_string()));
    }
    if contains("ALGERIA") {
        return Some(Currency::from("DZD".to_string()));
    }
    if contains("TUNISIA") {
        return Some(Currency::from("TND".to_string()));
    }
    if contains("ARGENTINA") {
        return Some(Currency::from("ARS".to_string()));
    }
    if contains("CHILE") {
        return Some(Currency::from("CLP".to_string()));
    }
    if contains("COLOMBIA") {
        return Some(Currency::from("COP".to_string()));
    }
    if contains("PERU") {
        return Some(Currency::from("PEN".to_string()));
    }
    if contains("URUGUAY") {
        return Some(Currency::from("UYU".to_string()));
    }
    if contains("PARAGUAY") {
        return Some(Currency::from("PYG".to_string()));
    }
    if contains("BOLIVIA") {
        return Some(Currency::from("BOB".to_string()));
    }
    if contains("VENEZUELA") {
        return Some(Currency::from("VES".to_string()));
    }
    if contains("PANAMA") {
        return Some(Currency::USD);
    }
    if contains("ECUADOR") {
        return Some(Currency::USD);
    }
    if contains("EL SALVADOR") {
        return Some(Currency::USD);
    }
    if contains("BAHAMAS") {
        return Some(Currency::from("BSD".to_string()));
    }
    if contains("CAYMAN") {
        return Some(Currency::from("KYD".to_string()));
    }
    if contains("BERMUDA") {
        return Some(Currency::from("BMD".to_string()));
    }
    if contains("TRINIDAD") {
        return Some(Currency::from("TTD".to_string()));
    }
    if contains("JAMAICA") {
        return Some(Currency::from("JMD".to_string()));
    }
    if contains("BARBADOS") {
        return Some(Currency::from("BBD".to_string()));
    }
    if contains("DOMINICAN") {
        return Some(Currency::from("DOP".to_string()));
    }
    if contains("ICELAND") {
        return Some(Currency::from("ISK".to_string()));
    }
    if contains("POLAND") {
        return Some(Currency::PLN);
    }
    if contains("CZECH") {
        return Some(Currency::CZK);
    }
    if contains("HUNGARY") {
        return Some(Currency::HUF);
    }
    if contains("ROMANIA") {
        return Some(Currency::from("RON".to_string()));
    }
    if contains("BULGARIA") {
        return Some(Currency::from("BGN".to_string()));
    }
    if contains("UKRAINE") {
        return Some(Currency::from("UAH".to_string()));
    }
    if contains("BELARUS") {
        return Some(Currency::from("BYN".to_string()));
    }
    if contains("SERBIA") {
        return Some(Currency::from("RSD".to_string()));
    }
    if contains("GEORGIA") {
        return Some(Currency::from("GEL".to_string()));
    }
    if contains("ARMENIA") {
        return Some(Currency::from("AMD".to_string()));
    }
    if contains("AZERBAIJAN") {
        return Some(Currency::from("AZN".to_string()));
    }
    if contains("KAZAKHSTAN") {
        return Some(Currency::from("KZT".to_string()));
    }
    if contains("UZBEKISTAN") {
        return Some(Currency::from("UZS".to_string()));
    }
    if contains("TURKMENISTAN") {
        return Some(Currency::from("TMT".to_string()));
    }
    if contains("KYRGYZSTAN") {
        return Some(Currency::from("KGS".to_string()));
    }
    if contains("TAJIKISTAN") {
        return Some(Currency::from("TJS".to_string()));
    }
    if contains("MONGOLIA") {
        return Some(Currency::from("MNT".to_string()));
    }
    if contains("FIJI") {
        return Some(Currency::from("FJD".to_string()));
    }
    if contains("SAMOA") {
        return Some(Currency::from("WST".to_string()));
    }
    if contains("TONGA") {
        return Some(Currency::from("TOP".to_string()));
    }
    if contains("VANUATU") {
        return Some(Currency::from("VUV".to_string()));
    }
    if contains("SOLOMON") {
        return Some(Currency::from("SBD".to_string()));
    }
    if contains("PAPUA") {
        return Some(Currency::from("PGK".to_string()));
    }
    if contains("NIGER") {
        return Some(Currency::from("XOF".to_string()));
    }
    if contains("SENEGAL") {
        return Some(Currency::from("XOF".to_string()));
    }
    if contains("CAMEROON") {
        return Some(Currency::from("XAF".to_string()));
    }
    if contains("IVORY COAST") {
        return Some(Currency::from("XOF".to_string()));
    }
    if contains("COTE DIVOIRE") {
        return Some(Currency::from("XOF".to_string()));
    }
    if contains("ZAMBIA") {
        return Some(Currency::from("ZMW".to_string()));
    }
    if contains("ZIMBABWE") {
        return Some(Currency::from("ZWL".to_string()));
    }
    if contains("ANGOLA") {
        return Some(Currency::from("AOA".to_string()));
    }
    if contains("NAMIBIA") {
        return Some(Currency::from("NAD".to_string()));
    }
    if contains("BOTSWANA") {
        return Some(Currency::from("BWP".to_string()));
    }
    if contains("LESOTHO") {
        return Some(Currency::from("LSL".to_string()));
    }
    if contains("ESWATINI") || contains("SWAZILAND") {
        return Some(Currency::from("SZL".to_string()));
    }
    if contains("MOZAMBIQUE") {
        return Some(Currency::from("MZN".to_string()));
    }
    if contains("MADAGASCAR") {
        return Some(Currency::from("MGA".to_string()));
    }
    if contains("MAURITIUS") {
        return Some(Currency::from("MUR".to_string()));
    }
    if contains("MALAWI") {
        return Some(Currency::from("MWK".to_string()));
    }
    if contains("SEYCHELLES") {
        return Some(Currency::from("SCR".to_string()));
    }
    if contains("RWANDA") {
        return Some(Currency::from("RWF".to_string()));
    }
    if contains("BURUNDI") {
        return Some(Currency::from("BIF".to_string()));
    }
    if contains("UGANDA") {
        return Some(Currency::from("UGX".to_string()));
    }
    if contains("TANZANIA") {
        return Some(Currency::from("TZS".to_string()));
    }
    if contains("SOMALIA") {
        return Some(Currency::from("SOS".to_string()));
    }
    if contains("DJIBOUTI") {
        return Some(Currency::from("DJF".to_string()));
    }
    if contains("ERITREA") {
        return Some(Currency::from("ERN".to_string()));
    }

    None
}
