pub struct Metadata<'a> {
    pub(crate) lines: Vec<usize>,
    pub(crate) cmnts: Vec<(usize, bool, &'a str)>,
}

impl Metadata<'_> {
    pub fn line_index(&self, pos: usize) -> usize {
        match self.lines.binary_search(&(pos + 1)) {
            Ok(v) => v,
            Err(v) => v,
        }
    }

    pub fn find_comment(&self, pos: usize) -> Option<&(usize, bool, &'_ str)> {
        let tmp = self.line_index(pos);
        let start = match self.lines.get(tmp - 1) {
            Some(v) => *v,
            _ => 0,
        };
        let end = self.lines[tmp];

        if start > pos || pos >= end {
            return None;
        }

        let tmp = match self.cmnts.binary_search_by(|v| v.0.cmp(&pos)) {
            Ok(v) => v,
            Err(v) => v,
        };
        let Some(cmnt) = self.cmnts.get(tmp) else {
            return None;
        };

        if cmnt.0 > end {
            return self.cmnts.get(tmp - 1);
        }

        if cmnt.0 < start {
            return self.cmnts.get(tmp + 1);
        }

        return Some(cmnt);
    }
}
