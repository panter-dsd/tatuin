use clap::ValueEnum;

#[derive(Clone, PartialEq, Eq, ValueEnum, Debug)]
pub enum FilterState {
    Completed,
    Uncompleted,
    InProgress,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum)]
pub enum Due {
    Overdue,
    Today,
    Future,
    NoDate,
}

#[derive(Debug)]
pub struct Filter {
    pub states: Vec<FilterState>,
    pub due: Vec<Due>,
}
