use serde::{self, Deserialize, Deserializer, de};

/// Deserialize form kcs api form IDs
pub(crate) fn deserialize_form_ivec<'de, D>(deserializer: D) -> Result<Vec<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.split(',')
        .map(|part| {
            part.trim()
                .parse::<i64>()
                .map_err(|e| de::Error::custom(format!("cannot parse to int: {e}")))
        })
        .collect()
}

/// Deserialize a kcs api boolean form flag: the client only ever sends "0" or "1".
pub(crate) fn deserialize_form_flag<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        other => {
            Err(de::Error::custom(format!("expected form flag \"0\" or \"1\", got {other:?}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::IntoDeserializer;
    use serde::de::value::{Error, StrDeserializer};

    fn value(s: &str) -> StrDeserializer<'_, Error> {
        s.into_deserializer()
    }

    #[test]
    fn form_ivec_parses_comma_separated_ids() {
        assert_eq!(deserialize_form_ivec(value("1,2,3")).unwrap(), vec![1, 2, 3]);
        assert_eq!(deserialize_form_ivec(value("7")).unwrap(), vec![7]);
        assert_eq!(deserialize_form_ivec(value(" 4 , 5 ")).unwrap(), vec![4, 5]);
        assert!(deserialize_form_ivec(value("1,x")).is_err());
        assert!(deserialize_form_ivec(value("")).is_err());
    }

    #[test]
    fn form_flag_accepts_only_zero_and_one() {
        assert!(!deserialize_form_flag(value("0")).unwrap());
        assert!(deserialize_form_flag(value("1")).unwrap());
        assert!(deserialize_form_flag(value("2")).is_err());
        assert!(deserialize_form_flag(value("")).is_err());
        assert!(deserialize_form_flag(value("true")).is_err());
    }
}
