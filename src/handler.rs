use crate::event::SchedulerEvent;

pub trait SchedulerHandler<T> where T: PartialEq + Eq + Send + Sync + Clone
{
    fn tick(&self, event: SchedulerEvent<T>) -> impl std::future::Future<Output = ()>;
}