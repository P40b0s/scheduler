use std::fmt::{Debug, Display};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum RepeatingStrategy
{
    Once,
    Dialy,
    Forever,
    Monthly,
}
impl Display for RepeatingStrategy
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result 
    {
        match self
        {
            RepeatingStrategy::Once => f.write_str("once"),
            RepeatingStrategy::Forever => f.write_str("forever"),
            RepeatingStrategy::Dialy => f.write_str("dialy"),
            RepeatingStrategy::Monthly => f.write_str("monthly")
        }
    }
}
#[derive(Clone, Debug)]
pub struct Event<T> where Self: Send, T: PartialEq + Eq + Send + Clone
{
    pub id: T,
    pub current: u32,
    pub len: u32
}
impl<T> Event<T> where T: PartialEq + Eq + Send + Sync + Clone
{
    pub (crate) fn new(id: T, current: u32, len: u32) -> Self
    {
        Self
        {
            id,
            current,
            len
        }
    }
}


#[derive(Clone, Debug)]
pub enum SchedulerEvent<T> where T: PartialEq + Eq + Send + Sync + Clone
{
    ///time in task is expired and task is not repeatable
    Expired(T),
    ///event every 1 minute
    Tick(Event<T>),
    ///event on finish task, if task is not repeatable
    Finish(T),
    /// event on finish repeat cycle for task, task will rerun with new end time
    FinishCycle(Event<T>)
}
