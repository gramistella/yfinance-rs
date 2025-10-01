//! Helpers for inferring currencies from country information.

use std::{collections::HashMap, sync::LazyLock};

use paft::money::{Currency, IsoCurrency};

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
        let parsed = (*code).parse().unwrap_or(Currency::Iso(IsoCurrency::USD));
        map.insert(*country, parsed);
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
            ' ' | '\t' | '\n' | '\r' | '\'' | '`' | '"' => buf.push(' '),
            '-' | '_' | '/' | ',' | '.' | ';' | ':' | '&' | '(' | ')' | '[' | ']' | '{' | '}' => {
                buf.push(' ');
            }
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => {
                buf.push('A');
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
pub fn currency_for_country(country: &str) -> Option<Currency> {
    let normalized = normalize_country(country)?;

    if let Some(currency) = COUNTRY_TO_CURRENCY.get(normalized.as_str()) {
        return Some(currency.clone());
    }

    heuristic_currency_match(&normalized)
}

fn heuristic_currency_match(normalized: &str) -> Option<Currency> {
    match_americas(normalized)
        .or_else(|| match_europe(normalized))
        .or_else(|| match_asia_pacific(normalized))
        .or_else(|| match_mena(normalized))
        .or_else(|| match_caucasus_central_asia(normalized))
        .or_else(|| match_africa(normalized))
}

fn match_americas(s: &str) -> Option<Currency> {
    let c = |n| s.contains(n);
    if c("UNITED STATES") {
        return Some(Currency::Iso(IsoCurrency::USD));
    }
    if c("CANADA") {
        return Some(Currency::Iso(IsoCurrency::CAD));
    }
    if c("MEXICO") {
        return Some(Currency::Iso(IsoCurrency::MXN));
    }
    if c("BRAZIL") {
        return Some(Currency::Iso(IsoCurrency::BRL));
    }
    if c("ARGENTINA") {
        return "ARS".parse().ok();
    }
    if c("CHILE") {
        return "CLP".parse().ok();
    }
    if c("COLOMBIA") {
        return "COP".parse().ok();
    }
    if c("PERU") {
        return "PEN".parse().ok();
    }
    if c("URUGUAY") {
        return "UYU".parse().ok();
    }
    if c("PARAGUAY") {
        return "PYG".parse().ok();
    }
    if c("BOLIVIA") {
        return "BOB".parse().ok();
    }
    if c("VENEZUELA") {
        return "VES".parse().ok();
    }
    if c("PANAMA") || c("ECUADOR") || c("EL SALVADOR") {
        return Some(Currency::Iso(IsoCurrency::USD));
    }
    if c("BAHAMAS") {
        return "BSD".parse().ok();
    }
    if c("CAYMAN") {
        return "KYD".parse().ok();
    }
    if c("BERMUDA") {
        return "BMD".parse().ok();
    }
    if c("TRINIDAD") {
        return "TTD".parse().ok();
    }
    if c("JAMAICA") {
        return "JMD".parse().ok();
    }
    if c("BARBADOS") {
        return "BBD".parse().ok();
    }
    if c("DOMINICAN") {
        return "DOP".parse().ok();
    }
    Some(None?).or(None)
}

fn match_europe(s: &str) -> Option<Currency> {
    let c = |n| s.contains(n);
    if c("UNITED KINGDOM") || c("ENGLAND") || c("SCOTLAND") {
        return Some(Currency::Iso(IsoCurrency::GBP));
    }
    if c("EUROPEAN UNION") || c("EURO AREA") {
        return Some(Currency::Iso(IsoCurrency::EUR));
    }
    if c("SWITZERLAND") {
        return Some(Currency::Iso(IsoCurrency::CHF));
    }
    if c("NORWAY") {
        return Some(Currency::Iso(IsoCurrency::NOK));
    }
    if c("SWEDEN") {
        return Some(Currency::Iso(IsoCurrency::SEK));
    }
    if c("DENMARK") {
        return Some(Currency::Iso(IsoCurrency::DKK));
    }
    if c("ICELAND") {
        return "ISK".parse().ok();
    }
    if c("POLAND") {
        return Some(Currency::Iso(IsoCurrency::PLN));
    }
    if c("CZECH") {
        return Some(Currency::Iso(IsoCurrency::CZK));
    }
    if c("HUNGARY") {
        return Some(Currency::Iso(IsoCurrency::HUF));
    }
    if c("ROMANIA") {
        return "RON".parse().ok();
    }
    if c("BULGARIA") {
        return "BGN".parse().ok();
    }
    if c("UKRAINE") {
        return "UAH".parse().ok();
    }
    if c("BELARUS") {
        return "BYN".parse().ok();
    }
    if c("SERBIA") {
        return "RSD".parse().ok();
    }
    if c("TURKEY") {
        return Some(Currency::Iso(IsoCurrency::TRY));
    }
    Some(None?).or(None)
}

fn match_asia_pacific(s: &str) -> Option<Currency> {
    let c = |n| s.contains(n);
    if c("HONG KONG") {
        return Some(Currency::Iso(IsoCurrency::HKD));
    }
    if c("MACAU") {
        return "MOP".parse().ok();
    }
    if c("TAIWAN") {
        return "TWD".parse().ok();
    }
    if c("KOREA") {
        return Some(Currency::Iso(IsoCurrency::KRW));
    }
    if c("JAPAN") {
        return Some(Currency::Iso(IsoCurrency::JPY));
    }
    if c("CHINA") {
        return Some(Currency::Iso(IsoCurrency::CNY));
    }
    if c("INDIA") {
        return Some(Currency::Iso(IsoCurrency::INR));
    }
    if c("SINGAPORE") {
        return Some(Currency::Iso(IsoCurrency::SGD));
    }
    if c("MALAYSIA") {
        return Some(Currency::Iso(IsoCurrency::MYR));
    }
    if c("INDONESIA") {
        return Some(Currency::Iso(IsoCurrency::IDR));
    }
    if c("PHILIPPINES") {
        return Some(Currency::Iso(IsoCurrency::PHP));
    }
    if c("VIETNAM") {
        return Some(Currency::Iso(IsoCurrency::VND));
    }
    if c("THAILAND") {
        return Some(Currency::Iso(IsoCurrency::THB));
    }
    if c("LAOS") {
        return "LAK".parse().ok();
    }
    if c("CAMBODIA") {
        return "KHR".parse().ok();
    }
    if c("BRUNEI") {
        return "BND".parse().ok();
    }
    if c("MONGOLIA") {
        return "MNT".parse().ok();
    }
    if c("AUSTRALIA") {
        return Some(Currency::Iso(IsoCurrency::AUD));
    }
    if c("NEW ZEALAND") {
        return Some(Currency::Iso(IsoCurrency::NZD));
    }
    if c("FIJI") {
        return "FJD".parse().ok();
    }
    if c("SAMOA") {
        return "WST".parse().ok();
    }
    if c("TONGA") {
        return "TOP".parse().ok();
    }
    if c("VANUATU") {
        return "VUV".parse().ok();
    }
    if c("SOLOMON") {
        return "SBD".parse().ok();
    }
    if c("PAPUA") {
        return "PGK".parse().ok();
    }
    Some(None?).or(None)
}

fn match_mena(s: &str) -> Option<Currency> {
    let c = |n| s.contains(n);
    if c("ISRAEL") {
        return Some(Currency::Iso(IsoCurrency::ILS));
    }
    if c("SAUDI ARABIA") {
        return "SAR".parse().ok();
    }
    if c("UNITED ARAB EMIRATES") {
        return "AED".parse().ok();
    }
    if c("QATAR") {
        return "QAR".parse().ok();
    }
    if c("KUWAIT") {
        return "KWD".parse().ok();
    }
    if c("BAHRAIN") {
        return "BHD".parse().ok();
    }
    if c("OMAN") {
        return "OMR".parse().ok();
    }
    if c("EGYPT") {
        return "EGP".parse().ok();
    }
    if c("JORDAN") {
        return "JOD".parse().ok();
    }
    if c("LEBANON") {
        return "LBP".parse().ok();
    }
    if c("IRAQ") {
        return "IQD".parse().ok();
    }
    if c("IRAN") {
        return "IRR".parse().ok();
    }
    if c("AFGHANISTAN") {
        return "AFN".parse().ok();
    }
    if c("SYRIA") {
        return "SYP".parse().ok();
    }
    if c("YEMEN") {
        return "YER".parse().ok();
    }
    Some(None?).or(None)
}

fn match_caucasus_central_asia(s: &str) -> Option<Currency> {
    let c = |n| s.contains(n);
    if c("GEORGIA") {
        return "GEL".parse().ok();
    }
    if c("ARMENIA") {
        return "AMD".parse().ok();
    }
    if c("AZERBAIJAN") {
        return "AZN".parse().ok();
    }
    if c("KAZAKHSTAN") {
        return "KZT".parse().ok();
    }
    if c("UZBEKISTAN") {
        return "UZS".parse().ok();
    }
    if c("TURKMENISTAN") {
        return "TMT".parse().ok();
    }
    if c("KYRGYZSTAN") {
        return "KGS".parse().ok();
    }
    if c("TAJIKISTAN") {
        return "TJS".parse().ok();
    }
    Some(None?).or(None)
}

fn match_africa(s: &str) -> Option<Currency> {
    let c = |n| s.contains(n);
    if c("SOUTH AFRICA") {
        return Some(Currency::Iso(IsoCurrency::ZAR));
    }
    if c("NIGERIA") {
        return "NGN".parse().ok();
    }
    if c("GHANA") {
        return "GHS".parse().ok();
    }
    if c("KENYA") {
        return "KES".parse().ok();
    }
    if c("MOROCCO") {
        return "MAD".parse().ok();
    }
    if c("ALGERIA") {
        return "DZD".parse().ok();
    }
    if c("TUNISIA") {
        return "TND".parse().ok();
    }
    if c("ZAMBIA") {
        return "ZMW".parse().ok();
    }
    if c("ZIMBABWE") {
        return "ZWL".parse().ok();
    }
    if c("ANGOLA") {
        return "AOA".parse().ok();
    }
    if c("NAMIBIA") {
        return "NAD".parse().ok();
    }
    if c("BOTSWANA") {
        return "BWP".parse().ok();
    }
    if c("LESOTHO") {
        return "LSL".parse().ok();
    }
    if c("ESWATINI") || c("SWAZILAND") {
        return "SZL".parse().ok();
    }
    if c("MOZAMBIQUE") {
        return "MZN".parse().ok();
    }
    if c("MADAGASCAR") {
        return "MGA".parse().ok();
    }
    if c("MAURITIUS") {
        return "MUR".parse().ok();
    }
    if c("MALAWI") {
        return "MWK".parse().ok();
    }
    if c("SEYCHELLES") {
        return "SCR".parse().ok();
    }
    if c("RWANDA") {
        return "RWF".parse().ok();
    }
    if c("BURUNDI") {
        return "BIF".parse().ok();
    }
    if c("UGANDA") {
        return "UGX".parse().ok();
    }
    if c("TANZANIA") {
        return "TZS".parse().ok();
    }
    if c("SOMALIA") {
        return "SOS".parse().ok();
    }
    if c("DJIBOUTI") {
        return "DJF".parse().ok();
    }
    if c("ERITREA") {
        return "ERN".parse().ok();
    }
    if c("NIGER") || c("SENEGAL") || c("IVORY COAST") || c("COTE DIVOIRE") {
        return "XOF".parse().ok();
    }
    if c("CAMEROON") {
        return "XAF".parse().ok();
    }
    Some(None?).or(None)
}
