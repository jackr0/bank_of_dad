use regex::Regex;
use serde::{
    de::{self},
    Serialize,
};
use std::{fmt::Display, ops::Add};

use serde::{Deserialize, Deserializer, Serializer};
use serde_json::Number;

const MIN_AMOUNT_POUNDS: i64 = i64::MIN / 100;
const MAX_AMOUNT_POUNDS: i64 = i64::MAX / 100;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Amount {
    amount: i64,
}

impl Amount {
    pub fn from_pence(amount: i64) -> Amount {
        Amount { amount }
    }

    pub fn is_positive_nonzero(&self) -> bool {
        self.amount > 0
    }

    pub fn is_negative(&self) -> bool {
        self.amount < 0
    }

    pub fn serialize_for_db(&self) -> i64 {
        self.amount
    }

    pub fn deserialize_from_db(amount: i64) -> Amount {
        Amount { amount }
    }

    pub fn negate(&self) -> Amount {
        Amount {
            amount: self.amount * -1,
        }
    }
}

impl Add for Amount {
    type Output = Amount;

    fn add(self, rhs: Self) -> Self::Output {
        Amount {
            amount: self.amount + rhs.amount,
        }
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.amount < 0 { "-" } else { "" };
        let pence = self.amount.abs() % 100;
        let pounds = self.amount.abs() / 100;
        write!(f, "{}{}.{:0>2}", sign, pounds, pence)
    }
}

impl Serialize for Amount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let number: Number = serde_json::from_str(&self.to_string()).unwrap();
        Number::serialize(&number, serializer)
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Amount, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n = Number::deserialize(deserializer)?;

        if let Some(v) = n.as_i64() {
            if v <= MIN_AMOUNT_POUNDS || v >= MAX_AMOUNT_POUNDS {
                return Err(de::Error::custom("Invalid amount"));
            } else {
                return Ok(Amount::from_pence(v * 100));
            }
        }

        if let Some(v) = n.as_u64() {
            if v >= MAX_AMOUNT_POUNDS as u64 {
                return Err(de::Error::custom("Invalid amount"));
            } else {
                match i64::try_from(v) {
                    Ok(v) => {
                        return Ok(Amount::from_pence(v * 100));
                    }
                    Err(_) => {
                        return Err(de::Error::custom("Failed to parse"));
                    }
                }
            }
        }

        let re = Regex::new(r"^(\-?[0-9]+)(\.([0-9]{2}))?$").unwrap();
        if let Some(caps) = re.captures(n.as_str()) {
            if caps.len() == 4 {
                if let Ok(pounds_value) = i64::from_str_radix(&caps[1], 10) {
                    if pounds_value <= MIN_AMOUNT_POUNDS || pounds_value >= MAX_AMOUNT_POUNDS {
                        return Err(de::Error::custom("Invalid amount"));
                    } else {
                        if let Ok(mut pence_value) = i64::from_str_radix(&caps[3], 10) {
                            if caps[1].starts_with('-') {
                                pence_value *= -1;
                            }
                            return Ok(Amount::from_pence((pounds_value * 100) + pence_value));
                        }
                    }
                }
            }
        }

        Err(de::Error::custom("Failed to parse"))
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::model::amount::Amount;

    #[derive(Deserialize, Debug, Serialize)]
    struct TestStruct {
        amount: Amount,
    }

    #[test]
    fn fmt_test() {
        assert_eq!(Amount::from_pence(0).to_string(), "0.00");
        assert_eq!(Amount::from_pence(12).to_string(), "0.12");
        assert_eq!(Amount::from_pence(1234).to_string(), "12.34");
        assert_eq!(Amount::from_pence(7777).to_string(), "77.77");
        assert_eq!(Amount::from_pence(-12).to_string(), "-0.12");
        assert_eq!(Amount::from_pence(-1234).to_string(), "-12.34");
        assert_eq!(Amount::from_pence(-7777).to_string(), "-77.77");
    }

    #[test]
    fn deserialize_test() {
        assert_eq!(
            Amount::from_pence(0),
            serde_json::from_str::<TestStruct>(r#"{"amount":0}"#)
                .unwrap()
                .amount
        );
        assert_eq!(
            Amount::from_pence(0),
            serde_json::from_str::<TestStruct>(r#"{"amount":0.00}"#)
                .unwrap()
                .amount
        );
        assert_eq!(
            Amount::from_pence(1),
            serde_json::from_str::<TestStruct>(r#"{"amount":0.01}"#)
                .unwrap()
                .amount
        );
        assert_eq!(
            Amount::from_pence(-1),
            serde_json::from_str::<TestStruct>(r#"{"amount":-0.01}"#)
                .unwrap()
                .amount
        );

        assert_eq!(
            Amount::from_pence(6700),
            serde_json::from_str::<TestStruct>(r#"{"amount":67}"#)
                .unwrap()
                .amount
        );
        assert_eq!(
            Amount::from_pence(-6700),
            serde_json::from_str::<TestStruct>(r#"{"amount":-67}"#)
                .unwrap()
                .amount
        );
        assert_eq!(
            Amount::from_pence(6789),
            serde_json::from_str::<TestStruct>(r#"{"amount":67.89}"#)
                .unwrap()
                .amount
        );
        assert_eq!(
            Amount::from_pence(-6789),
            serde_json::from_str::<TestStruct>(r#"{"amount":-67.89}"#)
                .unwrap()
                .amount
        );

        assert_eq!(
            Amount::from_pence(-9223372036854775700),
            serde_json::from_str::<TestStruct>(r#"{"amount":-92233720368547757}"#)
                .unwrap()
                .amount
        );

        assert_eq!(
            Amount::from_pence(-9223372036854775799),
            serde_json::from_str::<TestStruct>(r#"{"amount":-92233720368547757.99}"#)
                .unwrap()
                .amount
        );

        assert_eq!(
            Amount::from_pence(9223372036854775700),
            serde_json::from_str::<TestStruct>(r#"{"amount":92233720368547757}"#)
                .unwrap()
                .amount
        );

        assert_eq!(
            Amount::from_pence(9223372036854775799),
            serde_json::from_str::<TestStruct>(r#"{"amount":92233720368547757.99}"#)
                .unwrap()
                .amount
        );

        assert!(
            serde_json::from_str::<TestStruct>(r#"{"amount":-92233720368547758}"#)
                .is_err_and(|e| e.to_string().contains("Invalid amount"))
        );

        assert!(
            serde_json::from_str::<TestStruct>(r#"{"amount":92233720368547758}"#)
                .is_err_and(|e| e.to_string().contains("Invalid amount"))
        );

        assert!(
            serde_json::from_str::<TestStruct>(r#"{"amount":92233720368547758.01}"#)
                .is_err_and(|e| e.to_string().contains("Invalid amount"))
        );

        assert!(
            serde_json::from_str::<TestStruct>(r#"{"amount":-92233720368547758.01}"#)
                .is_err_and(|e| e.to_string().contains("Invalid amount"))
        );

        assert!(serde_json::from_str::<TestStruct>(r#"{"amount":10.0}"#)
            .is_err_and(|e| e.to_string().contains("Failed to parse")));

        assert!(serde_json::from_str::<TestStruct>(r#"{"amount":0.0a}"#)
            .is_err_and(|e| e.to_string().contains("Failed to parse")));

        assert!(serde_json::from_str::<TestStruct>(r#"{"amount":"0.0"}"#)
            .is_err_and(|e| e.to_string().contains("invalid type: string")));
    }

    #[test]
    fn serialize_test() {
        assert_eq!(
            r#"{"amount":0.00}"#,
            serde_json::to_string(&TestStruct {
                amount: Amount::from_pence(0)
            })
            .unwrap()
        );

        assert_eq!(
            r#"{"amount":12.34}"#,
            serde_json::to_string(&TestStruct {
                amount: Amount::from_pence(1234)
            })
            .unwrap()
        );

        assert_eq!(
            r#"{"amount":-12.34}"#,
            serde_json::to_string(&TestStruct {
                amount: Amount::from_pence(-1234)
            })
            .unwrap()
        );

        assert_eq!(
            r#"{"amount":12.00}"#,
            serde_json::to_string(&TestStruct {
                amount: Amount::from_pence(1200)
            })
            .unwrap()
        );

        assert_eq!(
            r#"{"amount":-12.00}"#,
            serde_json::to_string(&TestStruct {
                amount: Amount::from_pence(-1200)
            })
            .unwrap()
        );

        assert_eq!(
            r#"{"amount":0.12}"#,
            serde_json::to_string(&TestStruct {
                amount: Amount::from_pence(12)
            })
            .unwrap()
        );

        assert_eq!(
            r#"{"amount":-0.12}"#,
            serde_json::to_string(&TestStruct {
                amount: Amount::from_pence(-12)
            })
            .unwrap()
        );
    }
}
