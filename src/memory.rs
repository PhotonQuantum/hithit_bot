use eyre::{ContextCompat, Report};
use lru_cache::LruCache;
use teloxide::types::{ChatId, Message, MessageId, User};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MessageMeta {
    pub chat_id: ChatId,
    pub message_id: MessageId,
    pub sender: User,
}

impl TryFrom<Message> for MessageMeta {
    type Error = Report;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        Ok(Self {
            chat_id: msg.chat.id,
            message_id: msg.id,
            sender: msg.from.wrap_err("failed to get sender from message")?,
        })
    }
}

impl TryFrom<&Message> for MessageMeta {
    type Error = Report;

    fn try_from(msg: &Message) -> Result<Self, Self::Error> {
        Ok(Self {
            chat_id: msg.chat.id,
            message_id: msg.id,
            sender: msg
                .from
                .clone()
                .wrap_err("failed to get sender from message")?,
        })
    }
}

pub struct ReplyBooking {
    forward_map: LruCache<MessageMeta, MessageMeta>,
    reverse_map: LruCache<MessageMeta, MessageMeta>,
}

impl ReplyBooking {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            forward_map: LruCache::new(capacity),
            reverse_map: LruCache::new(capacity),
        }
    }
    pub fn book(&mut self, replied_to: MessageMeta, reply: MessageMeta) {
        self.forward_map.insert(replied_to.clone(), reply.clone());
        self.reverse_map.insert(reply, replied_to);
    }
    pub fn forward_lookup(&mut self, replied_to: &MessageMeta) -> Option<&MessageMeta> {
        self.forward_map.get_mut(replied_to).map(|m| &*m)
    }
    pub fn reverse_lookup(&mut self, reply: &MessageMeta) -> Option<&MessageMeta> {
        self.reverse_map.get_mut(reply).map(|m| &*m)
    }
    pub fn forget(&mut self, replied_to: &MessageMeta) {
        if let Some(reply) = self.forward_map.remove(replied_to) {
            self.reverse_map.remove(&reply);
        }
    }
}
