
#[derive(Serialize, Deserialize)]
pub struct Block {
    pub number: u64,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum BlockError { NoSuchBlock }