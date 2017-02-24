
#[derive(Serialize, Deserialize)]
pub struct Block {
    number: u64,
    data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub enum BlockError { NoSuchBlock }