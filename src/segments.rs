use std::borrow::Borrow;
use std::collections::{Bound, HashMap, HashSet, VecDeque};
use std::ops::{Deref, DerefMut, RangeBounds};

use maplit::hashset;
use ranges::Ranges;
use teloxide::types::{MessageEntity, MessageEntityKind, User};

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct Segment {
    pub kind: HashSet<MessageEntityKind>,
    pub text: String,
}

impl Segment {
    pub fn empty() -> Self {
        Self {
            kind: HashSet::new(),
            text: String::from(" "),
        }
    }
    pub fn from_user_with_name(user: User, name: String) -> Self {
        Self {
            text: name,
            kind: hashset!(MessageEntityKind::TextMention { user }),
        }
    }
    pub fn from_user(user: User) -> Self {
        Self::from_user_with_name(
            user.clone(),
            if let Some(last_name) = &user.last_name {
                format!("{} {}", user.first_name, last_name)
            } else {
                user.first_name
            },
        )
    }
}

impl<T: Borrow<User>> From<T> for Segment {
    fn from(user: T) -> Self {
        Self::from_user(user.borrow().clone())
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct Segments {
    data: VecDeque<Segment>,
}

impl Segments {
    pub const fn new(data: VecDeque<Segment>) -> Self {
        Self { data }
    }
}

impl<T: IntoIterator<Item = Segment>> From<T> for Segments {
    fn from(data: T) -> Self {
        Self {
            data: data.into_iter().collect(),
        }
    }
}

impl Deref for Segments {
    type Target = VecDeque<Segment>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Segments {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Segments {
    pub fn build(text: &str, entities: &[MessageEntity]) -> Self {
        let text_utf16: Vec<_> = text.encode_utf16().collect();

        let mut ranges: Vec<_> = entities
            .iter()
            .map(|entity| EntityRange {
                kind: Some(entity.kind.clone()),
                start: entity.offset,
                end: entity.offset + entity.length,
            })
            .collect();
        ranges.sort_unstable_by(|a, b| {
            usize::cmp(&a.start, &b.start)
                .then(usize::cmp(&a.end, &b.end).reverse())
                .reverse()
        });

        let mut offset: usize = 0;
        let mut segments: VecDeque<Segment> = VecDeque::with_capacity(entities.len());
        let mut stack: Vec<EntityRange> = Vec::with_capacity(entities.len());
        stack.push(EntityRange {
            kind: None,
            start: 0,
            end: text_utf16.len(),
        });

        while let Some(next) = ranges.last() {
            let curr = stack.last().unwrap();
            if next.start >= curr.end {
                if offset < curr.end {
                    segments.push_back(Segment {
                        kind: kinds(&stack),
                        text: String::from_utf16_lossy(&text_utf16[offset..curr.end]),
                    });
                }
                offset = curr.end;
                stack.pop().unwrap();
            } else {
                if next.start > offset {
                    segments.push_back(Segment {
                        kind: kinds(&stack),
                        text: String::from_utf16_lossy(&text_utf16[offset..next.start]),
                    });
                    offset = next.start;
                }
                stack.push(next.clone());
                ranges.pop();
            }
        }
        while let Some(curr) = stack.last() {
            segments.push_back(Segment {
                kind: kinds(&stack),
                text: String::from_utf16_lossy(&text_utf16[offset..curr.end]),
            });
            offset = curr.end;
            stack.pop();
        }

        Self { data: segments }
    }

    pub fn drain_head(mut self, length: usize) -> Option<Self> {
        let mut to_drain = length;
        while to_drain > 0 && !self.data.is_empty() {
            let front = self.data.front_mut().unwrap();
            let text = &front.text;
            let data_len = text.chars().count();
            if data_len >= to_drain {
                front.text = front.text.chars().skip(to_drain).collect();
                to_drain = 0;
            } else {
                self.data.pop_front();
                to_drain -= data_len;
            }
        }

        if to_drain == 0 {
            Some(self)
        } else {
            None
        }
    }

    pub fn trim_start(mut self) -> Self {
        while let Some(front) = self.data.front_mut() {
            let trimmed = front.text.trim_start().to_string();
            if trimmed.is_empty() {
                self.data.pop_front();
            } else {
                front.text = trimmed;
                break;
            }
        }
        self
    }

    pub fn trim_end(mut self) -> Self {
        while let Some(back) = self.data.back_mut() {
            let trimmed = back.text.trim_end().to_string();
            if trimmed.is_empty() {
                self.data.pop_back();
            } else {
                back.text = trimmed;
                break;
            }
        }
        self
    }

    pub fn trim(self) -> Self {
        self.trim_start().trim_end()
    }

    pub fn text(&self) -> String {
        self.data.iter().fold(String::new(), |mut base, segment| {
            base.push_str(segment.text.as_str());
            base
        })
    }

    pub fn entities(&self) -> Vec<MessageEntity> {
        let mut offset: usize = 0;
        let mut entity_buckets: HashMap<MessageEntityKind, Ranges<usize>> = HashMap::new();
        for segment in &self.data {
            let length = segment.text.encode_utf16().count();
            for kind in &segment.kind {
                if !entity_buckets.contains_key(kind) {
                    entity_buckets.insert(kind.clone(), Ranges::new());
                }
                entity_buckets
                    .get_mut(kind)
                    .unwrap()
                    .insert(offset..offset + length);
            }
            offset += length;
        }

        entity_buckets
            .into_iter()
            .flat_map(|(kind, ranges)| {
                ranges
                    .as_slice()
                    .iter()
                    .map(|range| {
                        let start = *unwrap_bound(range.start_bound()).unwrap();
                        let end = *unwrap_bound(range.end_bound()).unwrap();
                        MessageEntity::new(kind.clone(), start, end - start)
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

#[derive(Clone)]
struct EntityRange {
    kind: Option<MessageEntityKind>,
    start: usize,
    end: usize,
}

fn kinds(stack: &[EntityRange]) -> HashSet<MessageEntityKind> {
    stack
        .iter()
        .filter_map(|range| range.kind.clone())
        .collect()
}

const fn unwrap_bound<T>(bound: Bound<&T>) -> Option<&T> {
    match bound {
        Bound::Included(v) | Bound::Excluded(v) => Some(v),
        Bound::Unbounded => None,
    }
}
