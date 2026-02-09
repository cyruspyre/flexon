use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitmCatalog<'a> {
    #[serde(borrow)]
    area_names: HashMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(borrow)]
    audience_sub_category_names: HashMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(borrow)]
    block_names: HashMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(borrow)]
    events: HashMap<Cow<'a, str>, Event<'a>>,
    performances: Vec<Performance<'a>>,
    #[serde(borrow)]
    seat_category_names: HashMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(borrow)]
    sub_topic_names: HashMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(borrow)]
    subject_names: HashMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(borrow)]
    topic_names: HashMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(borrow)]
    topic_sub_topics: HashMap<Cow<'a, str>, Vec<u64>>,
    #[serde(borrow)]
    venue_names: HashMap<Cow<'a, str>, Cow<'a, str>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event<'a> {
    description: (),
    id: u64,
    #[serde(borrow)]
    logo: Option<Cow<'a, str>>,
    #[serde(borrow)]
    name: Cow<'a, str>,
    sub_topic_ids: Vec<u64>,
    subject_code: (),
    subtitle: (),
    topic_ids: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Performance<'a> {
    event_id: u64,
    id: u64,
    #[serde(borrow)]
    logo: Option<Cow<'a, str>>,
    name: (),
    prices: Vec<Price>,
    seat_categories: Vec<SeatCategory>,
    seat_map_image: (),
    start: u64,
    #[serde(borrow)]
    venue_code: Cow<'a, str>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    amount: u64,
    audience_sub_category_id: u64,
    seat_category_id: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatCategory {
    areas: Vec<Area>,
    seat_category_id: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Area {
    area_id: u64,
    block_ids: Vec<()>,
}
