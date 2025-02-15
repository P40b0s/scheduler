use crate::{time::Time, RepeatingStrategy};

#[derive(Debug)]
pub (crate) struct Task<T> 
    where T: PartialEq + Eq + Send + Clone
{
    pub (crate) id: T,
    ///interval in minutes
    pub (crate) interval: Option<u32>,
    ///target time in format on utilites::Date
    pub (crate) time: Option<Time>,
    ///task repeating strategy
    pub (crate) repeating_strategy: RepeatingStrategy,
    /// task is finished
    pub (crate) finished: bool,
}
