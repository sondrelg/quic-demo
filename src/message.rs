#[derive(Debug)]
pub enum Message {
    Hello,
    Shutdown,
}


impl From<String> for Message {
    fn from(value: String) -> Self {
        match value.as_str() {
            "0" => Message::Hello,
            "1" => Message::Shutdown,
            _ => unreachable!()
        }
    }
}

impl From<Message> for bytes::Bytes {
    fn from(value: Message) -> Self {
        let value = match value {
            Message::Hello => "0",
            Message::Shutdown => "1"
        };
        bytes::Bytes::from(value)
    }
}


impl<'a> Into<&'a [u8]> for &Message {
    fn into(self) -> &'a [u8] {
        match self {
            Message::Hello => b"0",
            Message::Shutdown => b"1"
        }
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        match data.as_slice() {
            b"0" => Message::Hello,
            b"1" => Message::Shutdown,
            _ => panic!("Invalid data for Message"),
        }
    }
}