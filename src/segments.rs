use std::collections::{Bound, HashMap, HashSet, VecDeque};
use std::ops::RangeBounds;

use ranges::Ranges;
use teloxide::types::{MessageEntity, MessageEntityKind, User};

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct Segment {
    pub kind: HashSet<MessageEntityKind>,
    pub text: String,
}

impl Segment {
    pub fn from_user_with_name(user: &User, name: String) -> Self {
        Segment {
            text: name,
            kind: hashset!(MessageEntityKind::TextMention { user: user.clone() }),
        }
    }
    pub fn from_user(user: &User) -> Self {
        Self::from_user_with_name(
            user,
            if let Some(last_name) = &user.last_name {
                format!("{} {}", user.first_name, last_name)
            } else {
                user.first_name.clone()
            },
        )
    }
}

impl From<&User> for Segment {
    fn from(user: &User) -> Self {
        Self::from_user(user)
    }
}

#[macro_export]
macro_rules! empty_segment {
    () => {
        Segment {
            kind: HashSet::new(),
            text: String::from(" "),
        };
    };
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct Segments {
    data: VecDeque<Segment>,
}

impl From<VecDeque<Segment>> for Segments {
    fn from(data: VecDeque<Segment>) -> Self {
        Segments { data }
    }
}

impl Segments {
    pub fn inner_ref(&self) -> &VecDeque<Segment> {
        &self.data
    }

    pub fn inner_mut(&mut self) -> &mut VecDeque<Segment> {
        &mut self.data
    }

    pub fn build(text: &str, entities: &[MessageEntity]) -> Segments {
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
            end: text.len(),
        });

        while let Some(next) = ranges.last() {
            let curr = stack.last().unwrap();
            if next.start >= curr.end {
                if offset < curr.end {
                    segments.push_back(Segment {
                        kind: kinds(&stack),
                        text: text.chars().skip(offset).take(curr.end - offset).collect(),
                    });
                }
                offset = curr.end;
                stack.pop().unwrap();
            } else {
                if next.start > offset {
                    segments.push_back(Segment {
                        kind: kinds(&stack),
                        text: text
                            .chars()
                            .skip(offset)
                            .take(next.start - offset)
                            .collect(),
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
                text: text.chars().skip(offset).take(curr.end - offset).collect(),
            });
            offset = curr.end;
            stack.pop();
        }

        Segments { data: segments }
    }

    pub fn drain_head(mut self, length: usize) -> Option<Self> {
        let mut to_drain = length;
        while to_drain > 0 && !self.data.is_empty() {
            let front = self.data.front_mut().unwrap();
            let text = &front.text;
            let data_len = text.len();
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
            } else if trimmed == front.text.as_str() {
                break;
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
            } else if trimmed == back.text.as_str() {
                break;
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
            let length = segment.text.chars().count();
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

fn unwrap_bound<T>(bound: Bound<&T>) -> Option<&T> {
    match bound {
        Bound::Included(v) => Some(v),
        Bound::Excluded(v) => Some(v),
        Bound::Unbounded => None,
    }
}
