pub struct Frame {
    pub class_id: u8,
    pub message_id: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(class_id: u8, message_id: u8, payload: Vec<u8>) -> Self {
        Self {
            class_id,
            message_id,
            payload,
        }
    }
}
