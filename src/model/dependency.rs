#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Dependency {
    Hard(String),
    Soft(String),
    //Conflict(String),
    IfThenHard(String, String),
    IfThenSoft(String, String),
    //IfThenConflict(String, String),
}

impl Dependency {
    pub fn get_name(&self) -> &String {
        match self {
            Dependency::Hard(name) => name,
            Dependency::Soft(name) => name,
            Dependency::IfThenHard(_, name) => name,
            Dependency::IfThenSoft(_, name) => name,
        }
    }
}
