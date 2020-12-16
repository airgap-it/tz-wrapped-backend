use base58check::{FromBase58Check, ToBase58Check};
use chrono::DateTime;
use super::micheline::TzError;

pub fn encode(value: &str, info: EncodingInfo, prefix: Option<&[u8]>) -> Result<Vec<u8>, TzError> {
    let (_, decoded) = value
        .from_base58check()
        .map_err(|_error| TzError::InvalidArgument)?;
    if decoded.len() <= info.prefix_bytes().len() || !decoded.starts_with(info.prefix_bytes()) {
        return Err(TzError::InvalidType);
    }
    let mut result = Vec::<u8>::new();
    if let Some(prefix) = prefix {
        result.extend_from_slice(prefix);
    }
    result.extend_from_slice(&decoded[info.prefix_bytes().len()..]);

    Ok(result)
}

pub fn decode(
    value: &Vec<u8>,
    info: EncodingInfo,
    remove_prefix: Option<&[u8]>,
) -> Result<String, TzError> {
    let mut encoded = &value[..];
    if let Some(prefix) = remove_prefix {
        if !encoded.starts_with(prefix) && encoded.len() <= prefix.len() {
            return Err(TzError::InvalidArgument);
        }
        encoded = &encoded[prefix.len()..];
    }
    let mut result = Vec::<u8>::new();
    result.extend_from_slice(info.prefix_bytes());
    result.extend_from_slice(encoded);

    Ok(result.to_base58check(info.version()))
}

pub fn encode_chain_id(value: &str) -> Result<Vec<u8>, TzError> {
    encode(value, NET, None)
}

pub fn encode_signature(value: &str) -> Result<Vec<u8>, TzError> {
    if value.starts_with(EncodingPrefix::EDSIG.prefix()) {
        return encode(value, EDSIG, None);
    }
    if value.starts_with(EncodingPrefix::SPSIG.prefix()) {
        return encode(value, SPSIG, None);
    }
    if value.starts_with(EncodingPrefix::P2SIG.prefix()) {
        return encode(value, P2SIG, None);
    }
    if value.starts_with(EncodingPrefix::SIG.prefix()) {
        return encode(value, SIG, None);
    }

    Err(TzError::InvalidArgument)
}

pub fn encode_pkh(value: &str, prefix: EncodingPrefix, tag: Option<&'static [u8]>) -> Result<Vec<u8>, TzError> {
    let mut prefix_bytes = Vec::<u8>::new();
    if let Some(tag) = tag {
        prefix_bytes.extend_from_slice(tag);
    }
    match prefix {
        EncodingPrefix::TZ1 => {
            prefix_bytes.extend_from_slice(&[0]);

            encode(value, TZ1, Some(&prefix_bytes))
        }
        EncodingPrefix::TZ2 => {
            prefix_bytes.extend_from_slice(&[1]);

            encode(value, TZ2, Some(&prefix_bytes))
        }
        EncodingPrefix::TZ3 => {
            prefix_bytes.extend_from_slice(&[2]);

            encode(value, TZ3, Some(&prefix_bytes))
        }
        _ => Err(TzError::InvalidArgument),
    }
}

pub fn encode_address(value: &str, tz_only: bool) -> Result<Vec<u8>, TzError> {
    if value.len() <= 3 {
        return Err(TzError::InvalidArgument);
    }
    let prefix_string = &value[..3];
    let prefix = EncodingPrefix::from(prefix_string)?;
    match prefix {
        EncodingPrefix::TZ1 | EncodingPrefix::TZ2 | EncodingPrefix::TZ3 => {
            let mut tag: Option<&'static [u8]> = None;
            if !tz_only {
                tag = Some(&[0]);
            }

            encode_pkh(value, prefix, tag)
        }
        EncodingPrefix::KT1 => {
            let mut tag: Option<&'static [u8]> = None;
            if !tz_only {
                tag = Some(&[1]);
            }

            let mut encoded = encode(value, KT1, tag)?;
            encoded.push(0);

            Ok(encoded)
        }
        _ => Err(TzError::InvalidType)
    }
}

pub fn encode_public_key(value: &str) -> Result<Vec<u8>, TzError> {
    if value.len() <= 4 {
        return Err(TzError::InvalidArgument);
    }
    let prefix_string = &value[..4];
    let prefix = EncodingPrefix::from(prefix_string)?;
    match prefix {
        EncodingPrefix::EDPK => encode(value, EDPK, Some(&[0])),
        EncodingPrefix::SPPK => encode(value, SPPK, Some(&[1])),
        EncodingPrefix::P2PK => encode(value, P2PK, Some(&[2])),
        _ => Err(TzError::InvalidType)
    }
}

pub fn encode_contract(value: &str) -> Result<Vec<u8>, TzError> {
    let components: Vec<&str> = value.split("%").collect();
    if components.len() > 2  {
        return Err(TzError::InvalidArgument)
    }

    let (address, entrypoint) = (components[0], if components.len() == 2 { components[1] } else { "default" });
    let mut result = encode_address(address, false)?;
    if entrypoint != "default" {
        result.extend_from_slice(entrypoint.as_bytes());
    }

    Ok(result)
}

pub fn encode_timestamp(value: &str) -> Result<i64, TzError> {
    let date_time = DateTime::parse_from_rfc3339(value).map_err(|_error| {
        TzError::InvalidArgument
    })?;

    Ok(date_time.timestamp())
}

pub struct EncodingInfo {
    prefix: EncodingPrefix,
    encoded_prefix: &'static [u8],
    encoded_length: usize,
}

impl EncodingInfo {
    fn version(&self) -> u8 {
        self.encoded_prefix[0]
    }

    fn prefix_bytes(&self) -> &'static [u8] {
        &self.encoded_prefix[1..]
    }
}

const TZ1: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::TZ1,
    encoded_prefix: &[6, 161, 159],
    encoded_length: 20,
};
const TZ2: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::TZ2,
    encoded_prefix: &[6, 161, 161],
    encoded_length: 20,
};
const TZ3: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::TZ3,
    encoded_prefix: &[6, 161, 164],
    encoded_length: 20,
};
const KT: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::KT,
    encoded_prefix: &[2, 90, 121],
    encoded_length: 20,
};
const KT1: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::KT1,
    encoded_prefix: &[2, 90, 121],
    encoded_length: 20,
};

const EDSK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::EDSK,
    encoded_prefix: &[43, 246, 78, 7],
    encoded_length: 64,
};
const EDSK2: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::EDSK2,
    encoded_prefix: &[13, 15, 58, 7],
    encoded_length: 32,
};
const SPSK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::SPSK,
    encoded_prefix: &[17, 162, 224, 201],
    encoded_length: 32,
};
const P2SK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::P2SK,
    encoded_prefix: &[16, 81, 238, 189],
    encoded_length: 32,
};

const EDPK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::EDPK,
    encoded_prefix: &[13, 15, 37, 217],
    encoded_length: 32,
};
const SPPK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::SPPK,
    encoded_prefix: &[3, 254, 226, 86],
    encoded_length: 33,
};
const P2PK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::P2PK,
    encoded_prefix: &[3, 178, 139, 127],
    encoded_length: 33,
};

const EDESK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::EDESK,
    encoded_prefix: &[7, 90, 60, 179, 41],
    encoded_length: 56,
};
const SPESK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::SPESK,
    encoded_prefix: &[0x09, 0xed, 0xf1, 0xae, 0x96],
    encoded_length: 56,
};
const P2ESK: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::P2ESK,
    encoded_prefix: &[0x09, 0x30, 0x39, 0x73, 0xab],
    encoded_length: 56,
};

const EDSIG: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::EDSIG,
    encoded_prefix: &[9, 245, 205, 134, 18],
    encoded_length: 64,
};
const SPSIG: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::SPSIG,
    encoded_prefix: &[13, 115, 101, 19, 63],
    encoded_length: 64,
};
const P2SIG: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::P2SIG,
    encoded_prefix: &[54, 240, 44, 52],
    encoded_length: 64,
};
const SIG: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::SIG,
    encoded_prefix: &[4, 130, 43],
    encoded_length: 64,
};

const NET: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::NET,
    encoded_prefix: &[87, 82, 0],
    encoded_length: 4,
};
const B: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::B,
    encoded_prefix: &[1, 52],
    encoded_length: 32,
};
const O: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::O,
    encoded_prefix: &[5, 116],
    encoded_length: 32,
};
const LO: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::LO,
    encoded_prefix: &[133, 233],
    encoded_length: 32,
};
const LLO: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::LLO,
    encoded_prefix: &[29, 159, 109],
    encoded_length: 32,
};
const P: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::P,
    encoded_prefix: &[2, 170],
    encoded_length: 32,
};
const CO: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::CO,
    encoded_prefix: &[79, 179],
    encoded_length: 32,
};
const ID: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::ID,
    encoded_prefix: &[153, 103],
    encoded_length: 16,
};

const EXPR: EncodingInfo = EncodingInfo {
    prefix: EncodingPrefix::EXPR,
    encoded_prefix: &[13, 44, 64, 27],
    encoded_length: 32,
};

pub enum EncodingPrefix {
    TZ1,
    TZ2,
    TZ3,
    KT,
    KT1,
    EDSK2,
    SPSK,
    P2SK,
    EDPK,
    SPPK,
    P2PK,
    EDESK,
    SPESK,
    P2ESK,
    EDSK,
    EDSIG,
    SPSIG,
    P2SIG,
    SIG,
    NET,
    NCE,
    B,
    O,
    LO,
    LLO,
    P,
    CO,
    ID,
    EXPR,
}

impl EncodingPrefix {
    fn from(value: &str) -> Result<EncodingPrefix, TzError> {
        Ok(match value {
            "tz1" => EncodingPrefix::TZ1,
            "tz2" => EncodingPrefix::TZ2,
            "tz3" => EncodingPrefix::TZ3,
            "KT" => EncodingPrefix::KT,
            "KT1" => EncodingPrefix::KT1,
            "edsk2" => EncodingPrefix::EDSK2,
            "spsk" => EncodingPrefix::SPSK,
            "p2sk" => EncodingPrefix::P2SK,
            "edpk" => EncodingPrefix::EDPK,
            "sppk" => EncodingPrefix::SPPK,
            "p2pk" => EncodingPrefix::P2PK,
            "edesk" => EncodingPrefix::EDESK,
            "spesk" => EncodingPrefix::SPESK,
            "p2esk" => EncodingPrefix::P2ESK,
            "edsk" => EncodingPrefix::EDSK,
            "edsig" => EncodingPrefix::EDSIG,
            "spsig" => EncodingPrefix::SPSIG,
            "p2sig" => EncodingPrefix::P2SIG,
            "sig" => EncodingPrefix::SIG,
            "Net" => EncodingPrefix::NET,
            "nce" => EncodingPrefix::NCE,
            "b" => EncodingPrefix::B,
            "o" => EncodingPrefix::O,
            "Lo" => EncodingPrefix::LO,
            "LLo" => EncodingPrefix::LLO,
            "P" => EncodingPrefix::P,
            "Co" => EncodingPrefix::CO,
            "id" => EncodingPrefix::ID,
            "expr" => EncodingPrefix::EXPR,
            _ => Err(TzError::InvalidArgument)?
        })
    }

    fn prefix(&self) -> &str {
        match self {
            EncodingPrefix::TZ1 => "tz1",
            EncodingPrefix::TZ2 => "tz2",
            EncodingPrefix::TZ3 => "tz3",
            EncodingPrefix::KT => "KT",
            EncodingPrefix::KT1 => "KT1",
            EncodingPrefix::EDSK2 => "edsk2",
            EncodingPrefix::SPSK => "spsk",
            EncodingPrefix::P2SK => "p2sk",
            EncodingPrefix::EDPK => "edpk",
            EncodingPrefix::SPPK => "sppk",
            EncodingPrefix::P2PK => "p2pk",
            EncodingPrefix::EDESK => "edesk",
            EncodingPrefix::SPESK => "spesk",
            EncodingPrefix::P2ESK => "p2esk",
            EncodingPrefix::EDSK => "edsk",
            EncodingPrefix::EDSIG => "edsig",
            EncodingPrefix::SPSIG => "spsig",
            EncodingPrefix::P2SIG => "p2sig",
            EncodingPrefix::SIG => "sig",
            EncodingPrefix::NET => "Net",
            EncodingPrefix::NCE => "nce",
            EncodingPrefix::B => "b",
            EncodingPrefix::O => "o",
            EncodingPrefix::LO => "Lo",
            EncodingPrefix::LLO => "LLo",
            EncodingPrefix::P => "P",
            EncodingPrefix::CO => "Co",
            EncodingPrefix::ID => "id",
            EncodingPrefix::EXPR => "expr",
        }
    }
}
