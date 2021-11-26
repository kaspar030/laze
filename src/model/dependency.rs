use std::fmt::Display;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Dependency<T>
where
    T: Display,
{
    Hard(T),
    Soft(T),
    //Conflict(String),
    IfThenHard(T, T),
    IfThenSoft(T, T),
    //IfThenConflict(String, String),
}

impl<T> Dependency<T>
where
    T: Display,
{
    pub fn get_name(&self) -> String {
        match self {
            Dependency::Hard(name) => name.to_string(),
            Dependency::Soft(name) => name.to_string(),
            Dependency::IfThenHard(_, name) => name.to_string(),
            Dependency::IfThenSoft(_, name) => name.to_string(),
        }
    }
}
