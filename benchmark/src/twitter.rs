use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

#[derive(Serialize, Deserialize)]
pub struct Twitter<'a> {
    #[serde(borrow)]
    statuses: Vec<Status<'a>>,
    search_metadata: SearchMetadata<'a>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchMetadata<'a> {
    completed_in: f64,
    max_id: u64,
    #[serde(borrow)]
    max_id_str: Cow<'a, str>,
    #[serde(borrow)]
    next_results: Cow<'a, str>,
    #[serde(borrow)]
    query: Cow<'a, str>,
    #[serde(borrow)]
    refresh_url: Cow<'a, str>,
    count: u64,
    since_id: u64,
    #[serde(borrow)]
    since_id_str: Cow<'a, str>,
}

#[derive(Serialize, Deserialize)]
pub struct Status<'a> {
    metadata: Metadata,
    #[serde(borrow)]
    created_at: Cow<'a, str>,
    id: u64,
    #[serde(borrow)]
    id_str: Cow<'a, str>,
    #[serde(borrow)]
    text: Cow<'a, str>,
    #[serde(borrow)]
    source: Cow<'a, str>,
    truncated: bool,
    in_reply_to_status_id: Option<u64>,
    #[serde(borrow)]
    in_reply_to_status_id_str: Option<Cow<'a, str>>,
    in_reply_to_user_id: Option<u64>,
    #[serde(borrow)]
    in_reply_to_user_id_str: Option<Cow<'a, str>>,
    #[serde(borrow)]
    in_reply_to_screen_name: Option<Cow<'a, str>>,
    user: User<'a>,
    geo: (),
    coordinates: (),
    place: (),
    contributors: (),
    retweet_count: u64,
    favorite_count: u64,
    entities: StatusEntities<'a>,
    favorited: bool,
    retweeted: bool,
    lang: Lang,
    retweeted_status: Option<Box<Status<'a>>>,
    possibly_sensitive: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct StatusEntities<'a> {
    hashtags: Vec<Hashtag<'a>>,
    symbols: Vec<()>,
    urls: Vec<Url<'a>>,
    #[serde(borrow)]
    user_mentions: Vec<UserMention<'a>>,
    media: Option<Vec<Media<'a>>>,
}

#[derive(Serialize, Deserialize)]
pub struct Hashtag<'a> {
    #[serde(borrow)]
    text: Cow<'a, str>,
    indices: [u64; 2],
}

#[derive(Serialize, Deserialize)]
pub struct Media<'a> {
    id: u64,
    #[serde(borrow)]
    id_str: Cow<'a, str>,
    indices: [u64; 2],
    #[serde(borrow)]
    media_url: Cow<'a, str>,
    #[serde(borrow)]
    media_url_https: Cow<'a, str>,
    #[serde(borrow)]
    url: Cow<'a, str>,
    #[serde(borrow)]
    display_url: Cow<'a, str>,
    #[serde(borrow)]
    expanded_url: Cow<'a, str>,
    #[serde(rename = "type", borrow)]
    media_type: Cow<'a, str>,
    sizes: Sizes,
    source_status_id: Option<f64>,
    #[serde(borrow)]
    source_status_id_str: Option<Cow<'a, str>>,
}

#[derive(Serialize, Deserialize)]
pub struct Sizes {
    medium: Size,
    small: Size,
    thumb: Size,
    large: Size,
}

#[derive(Serialize, Deserialize)]
pub struct Size {
    w: u16,
    h: u16,
    resize: Resize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resize {
    Crop,
    Fit,
}

#[derive(Serialize, Deserialize)]
pub struct Url<'a> {
    #[serde(borrow)]
    url: Cow<'a, str>,
    #[serde(borrow)]
    expanded_url: Cow<'a, str>,
    #[serde(borrow)]
    display_url: Cow<'a, str>,
    indices: [u64; 2],
}

#[derive(Serialize, Deserialize)]
pub struct UserMention<'a> {
    #[serde(borrow)]
    screen_name: Cow<'a, str>,
    #[serde(borrow)]
    name: Cow<'a, str>,
    id: u64,
    #[serde(borrow)]
    id_str: Cow<'a, str>,
    indices: [u64; 2],
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    Ja,
    Zh,
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    result_type: ResultType,
    iso_language_code: Lang,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    Recent,
}

#[derive(Serialize, Deserialize)]
pub struct User<'a> {
    id: u64,
    #[serde(borrow)]
    id_str: Cow<'a, str>,
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    screen_name: Cow<'a, str>,
    #[serde(borrow)]
    location: Cow<'a, str>,
    #[serde(borrow)]
    description: Cow<'a, str>,
    #[serde(borrow)]
    url: Option<Cow<'a, str>>,
    entities: UserEntities<'a>,
    protected: bool,
    followers_count: u64,
    friends_count: u64,
    listed_count: u64,
    #[serde(borrow)]
    created_at: Cow<'a, str>,
    favourites_count: u64,
    utc_offset: Option<i64>,
    #[serde(borrow)]
    time_zone: Option<Cow<'a, str>>,
    geo_enabled: bool,
    verified: bool,
    statuses_count: u64,
    lang: UserLang,
    contributors_enabled: bool,
    is_translator: bool,
    is_translation_enabled: bool,
    profile_background_color: Color,
    #[serde(borrow)]
    profile_background_image_url: Cow<'a, str>,
    #[serde(borrow)]
    profile_background_image_url_https: Cow<'a, str>,
    profile_background_tile: bool,
    #[serde(borrow)]
    profile_image_url: Cow<'a, str>,
    #[serde(borrow)]
    profile_image_url_https: Cow<'a, str>,
    #[serde(borrow)]
    profile_banner_url: Option<Cow<'a, str>>,
    profile_link_color: Color,
    profile_sidebar_border_color: Color,
    profile_sidebar_fill_color: Color,
    profile_text_color: Color,
    profile_use_background_image: bool,
    default_profile: bool,
    default_profile_image: bool,
    following: bool,
    follow_request_sent: bool,
    notifications: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UserEntities<'a> {
    #[serde(borrow)]
    url: Option<Description<'a>>,
    description: Description<'a>,
}

#[derive(Serialize, Deserialize)]
pub struct Description<'a> {
    #[serde(borrow)]
    urls: Vec<Url<'a>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UserLang {
    En,
    Es,
    It,
    Ja,
    #[serde(rename = "zh-cn")]
    Cn,
}

pub struct Color(u32);

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:06X}", self.0))
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(
            u32::from_str_radix(&String::deserialize(deserializer)?, 16)
                .map_err(serde::de::Error::custom)?,
        ))
    }
}
