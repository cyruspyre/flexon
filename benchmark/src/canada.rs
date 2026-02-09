use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize)]
pub struct Canada<'a> {
    #[serde(borrow, rename = "type")]
    canada_type: Cow<'a, str>,
    features: Vec<Feature<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct Feature<'a> {
    #[serde(borrow, rename = "type")]
    feature_type: Cow<'a, str>,
    properties: Properties<'a>,
    geometry: Geometry<'a>,
}

#[derive(Serialize, Deserialize)]
pub struct Geometry<'a> {
    #[serde(borrow, rename = "type")]
    geometry_type: Cow<'a, str>,
    coordinates: Vec<Vec<[f64; 2]>>,
}

#[derive(Serialize, Deserialize)]
pub struct Properties<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
}
