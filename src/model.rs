#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedMachine {
    pub module_name: String,
    pub states: Vec<String>,
    pub actions: Vec<Action>,
    pub invariants: Vec<String>,
    pub comments: Vec<Comment>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub comment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    pub target: String,
    pub text: String,
}

impl ParsedMachine {
    pub fn empty() -> Self {
        Self {
            module_name: "Untitled Machine".to_string(),
            states: Vec::new(),
            actions: Vec::new(),
            invariants: Vec::new(),
            comments: Vec::new(),
            warnings: Vec::new(),
        }
    }
}
