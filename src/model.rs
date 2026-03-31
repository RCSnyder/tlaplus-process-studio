#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedMachine {
    pub module_name: String,
    pub states: Vec<String>,
    pub init_state: Option<String>,
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
            init_state: None,
            actions: Vec::new(),
            invariants: Vec::new(),
            comments: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Return the initial state: either from `Init ==` or the first sorted state.
    pub fn start_state(&self) -> String {
        self.init_state.clone()
            .or_else(|| self.states.first().cloned())
            .unwrap_or_else(|| "No Spec".to_string())
    }
}
