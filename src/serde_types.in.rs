use std::ops::Deref;
use chrono::UTC;
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

// This single-element tuple struct is called a newtype struct.
#[derive(Debug)]
struct Date(chrono::Date<UTC>);

impl Deserialize for Date {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        struct Visitor;

        impl ::serde::de::Visitor for Visitor {
            type Value = Date;

            fn visit_str<E>(&mut self, value: &str) -> Result<Date, E>
                where E: ::serde::de::Error,
            {
                Ok(Date(chrono::Date::from_utc(chrono::naive::date::NaiveDate::from_str(value).unwrap(), UTC)))
            }
        }

        // Deserialize the enum from a string.
        deserializer.deserialize_str(Visitor)
    }
}

// Enable `Deref` coercion.
impl Deref for Date {
    type Target = chrono::Date<UTC>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize, Debug)]
struct Expense {
    date: String,
    lines: Vec<Line>,
}

#[derive(Deserialize, Debug)]
struct Line {
    #[serde(rename = "categoryType")]
    category_type: String,
    description: String,
    #[serde(rename = "assetDepreciation")]
    #[serde(default = "Vec::new")]
    asset_depreciation: Vec<AssetDepreciation>,
}

#[derive(Deserialize, Debug)]
struct AssetDepreciation {
    #[serde(rename = "depreciationCost")]
    depreciation_cost: f64,
    #[serde(rename = "depreciationDate")]
    depreciation_date: Date,
    #[serde(rename = "bookValue")]
    book_value: f64,
}

#[derive(Deserialize, Debug)]
struct AccessToken {
    access_token: String
}