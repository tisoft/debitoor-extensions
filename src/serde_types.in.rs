#[derive(Serialize, Deserialize, Debug)]
struct Expense {
    date: String,
    lines: Vec<Line>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Line {
    #[serde(rename = "categoryType")]
    category_type: String,
    description: String,
    #[serde(rename = "assetDepreciation")]
    #[serde(default = "Vec::new")]
    asset_depreciation: Vec<AssetDepreciation>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AssetDepreciation {
    depreciationCost: f64,
    depreciationDate: String,
    bookValue: f64,
}
