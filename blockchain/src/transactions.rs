use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub value: u64
} //какие поля еще нужно добавить?


impl Transaction {
    pub fn new(from: String, to: String, value: u64) -> Self {
        Transaction {
            from, 
            to,
            value
        }
    }
}
