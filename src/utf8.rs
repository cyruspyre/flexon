// taken from `utf8parse` crate at version `0.2.2`, messing with utf-8 validation
// is just beyond my scope. this is used for `skip_to()` to avoid requiring `alloc`.

pub struct Parser(State);

/// States the parser can be in.
///
/// There is a state for each initial input of the 3 and 4 byte sequences since
/// the following bytes are subject to different conditions than a tail byte.
#[allow(non_camel_case_types)]
enum State {
    /// Ground state; expect anything
    Ground = 0,
    /// 3 tail bytes
    Tail3 = 1,
    /// 2 tail bytes
    Tail2 = 2,
    /// 1 tail byte
    Tail1 = 3,
    /// UTF8-3 starting with E0
    U3_2_e0 = 4,
    /// UTF8-3 starting with ED
    U3_2_ed = 5,
    /// UTF8-4 starting with F0
    Utf8_4_3_f0 = 6,
    /// UTF8-4 starting with F4
    Utf8_4_3_f4 = 7,
}

impl Parser {
    #[inline]
    pub fn new() -> Parser {
        Parser(State::Ground)
    }

    #[inline]
    pub fn is_expecting(&self) -> bool {
        !matches!(self.0, State::Ground)
    }

    pub fn advance(&mut self, v: u8) -> bool {
        let (state, is_invalid) = match self.0 {
            State::Ground => match v {
                0x00..=0x7f => (State::Ground, false),
                0xc2..=0xdf => (State::Tail1, false),
                0xe0 => (State::U3_2_e0, false),
                0xe1..=0xec => (State::Tail2, false),
                0xed => (State::U3_2_ed, false),
                0xee..=0xef => (State::Tail2, false),
                0xf0 => (State::Utf8_4_3_f0, false),
                0xf1..=0xf3 => (State::Tail3, false),
                0xf4 => (State::Utf8_4_3_f4, false),
                _ => (State::Ground, true),
            },
            State::U3_2_e0 => match v {
                0xa0..=0xbf => (State::Tail1, false),
                _ => (State::Ground, true),
            },
            State::U3_2_ed => match v {
                0x80..=0x9f => (State::Tail1, false),
                _ => (State::Ground, true),
            },
            State::Utf8_4_3_f0 => match v {
                0x90..=0xbf => (State::Tail2, false),
                _ => (State::Ground, true),
            },
            State::Utf8_4_3_f4 => match v {
                0x80..=0x8f => (State::Tail2, false),
                _ => (State::Ground, true),
            },
            State::Tail3 => match v {
                0x80..=0xbf => (State::Tail2, false),
                _ => (State::Ground, true),
            },
            State::Tail2 => match v {
                0x80..=0xbf => (State::Tail1, false),
                _ => (State::Ground, true),
            },
            State::Tail1 => match v {
                0x80..=0xbf => (State::Ground, false),
                _ => (State::Ground, true),
            },
        };

        self.0 = state;
        is_invalid
    }
}
