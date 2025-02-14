use std::{fmt::{Debug, Display}, hash::Hash, sync::Arc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use utilites::Date;

#[derive(Debug)]
struct Time
{
    time: Date,
    current_timestramp: i64,
    finish_timestramp: i64,
    is_expired: bool,

}

impl Time
{
    pub fn new(date: Date) -> Self
    {
        let now = Date::now();
        let current_timestramp = now.as_naive_datetime().and_utc().timestamp();
        let finish_timestramp = time_diff(&now, &date);
        let is_expired = if finish_timestramp.is_negative() { true } else { false };
        Self
        {
            time: date,
            current_timestramp,
            finish_timestramp,
            is_expired
        }
    }
    pub fn update(&mut self)
    {
        let now = Date::now();
        let diff = time_diff(&now, &self.time);
        if diff.is_positive()
        {
            self.current_timestramp = self.finish_timestramp - diff;
        }
        else 
        {
            self.is_expired = true;  
        }
    }
    pub fn add_hours(&mut self, h: i64)
    {
        let new_date =  Date::now().with_time(&self.time).add_minutes(h*60);
        //if we take date now, nned correction with await timer (1 min)
        let old_date = Date::now().with_time(&self.time);
        //let current_timestramp = now.as_naive_datetime().and_utc().timestamp();
        let finish_timestramp = time_diff(&old_date, &new_date);
        self.current_timestramp = 0;
        self.finish_timestramp = finish_timestramp;
        self.is_expired = false;
        self.time = new_date;
    }
    pub fn add_months(&mut self, m: u32)
    {
        if let Some(new_date) =  Date::now().with_time(&self.time).add_months(m)
        {
            let old_date = Date::now().with_time(&self.time);
            let finish_timestramp = time_diff(&old_date, &new_date);
            self.current_timestramp = 0;
            self.finish_timestramp = finish_timestramp;
            self.is_expired = false;
            self.time = new_date;
        }
    }
}

#[derive(Debug)]
struct Task<T> 
    where T: PartialEq + Eq + Hash + Send + Clone
{
    id: T,
    ///interval in minutes
    interval: Option<u32>,
    ///target time in format on utilites::Date
    time: Option<Time>,
    ///task repeating strategy
    repeating_strategy: RepeatingStrategy,
    /// task is finished
    finished: bool,
}

///clone only cloning internal ref Arc<RwLock<Vec<Task>>>
#[derive(Debug)]
pub struct Scheduler<T>(Arc<RwLock<Vec<Task<T>>>>) where Self: Send + Sync, T: PartialEq + Eq + Hash + Send + Clone;

impl<T> Clone for Scheduler<T> where T: PartialEq + Eq + Hash + Send + Sync + Clone
{
    fn clone(&self) -> Self 
    {
        Self(self.0.clone())
    }
}

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
pub struct Event<T> where Self: Send, T: PartialEq + Eq + Hash + Send + Clone
{
    pub id: T,
    pub current: u32,
    pub len: u32
}
impl<T> Event<T> where T: PartialEq + Eq + Hash + Send + Sync + Clone
{
    fn new(id: T, current: u32, len: u32) -> Self
    {
        Self
        {
            id,
            current,
            len
        }
    }
}

pub trait SchedulerHandler<T> where T: PartialEq + Eq + Hash + Send + Sync + Clone
{
    fn tick(&self, event: SchedulerEvent<T>) -> impl std::future::Future<Output = ()>;
}


#[derive(Clone, Debug)]
pub enum SchedulerEvent<T> where T: PartialEq + Eq + Hash + Send + Sync + Clone
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

impl<T> Scheduler<T> where T: PartialEq + Eq + Hash + Send + Sync + Clone + Debug
{
    pub fn new() -> Self
    {
        Self(Arc::new(RwLock::new(Vec::new())))
    }
    pub async fn run<H: SchedulerHandler<T>>(&self, handler: H)
    {
        let mut minutes: u32 = 0;
        loop 
        {
            let mut guard = self.0.write().await;
            for task in guard.iter_mut()
            {
                match task.repeating_strategy
                {
                    RepeatingStrategy::Once =>
                    {
                        if let Some(interval) = task.interval.as_ref()
                        {
                            if minutes != 0
                            {
                                let div = minutes % interval;
                                if div == 0
                                {
                                    handler.tick(SchedulerEvent::Finish(task.id.clone())).await;
                                    task.finished = true;
                                }
                                else
                                {
                                    let event = Event::new(task.id.clone(), div, *interval);
                                    let _ = handler.tick(SchedulerEvent::Tick(event)).await;
                                }
                            }
                        }
                        else if let Some(time) = task.time.as_mut()
                        {
                            time.update();
                            if minutes == 0 && time.is_expired
                            {
                                handler.tick(SchedulerEvent::Expired(task.id.clone())).await;
                                task.finished = true;
                            }
                            else if minutes != 0
                            {
                                if time.is_expired
                                {
                                    handler.tick(SchedulerEvent::Finish(task.id.clone())).await;
                                    task.finished = true;
                                }
                                else
                                {
                                    let event = Event::new(task.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32);
                                    let _ = handler.tick(SchedulerEvent::Tick(event)).await;
                                }
                            }
                        }
                    },
                    RepeatingStrategy::Dialy | RepeatingStrategy::Forever =>
                    {
                        if let Some(interval) = task.interval.as_ref()
                        {
                            if minutes != 0
                            {
                                let div = minutes % interval;
                                if div == 0
                                {
                                    let event = Event::new(task.id.clone(), div, *interval);
                                    let _ = handler.tick(SchedulerEvent::FinishCycle(event)).await;
                                }
                                else
                                {
                                    let event = Event::new(task.id.clone(), div, *interval);
                                    let _ = handler.tick(SchedulerEvent::Tick(event)).await;
                                }
                            }
                        }
                        else if let Some(time) = task.time.as_mut()
                        {
                            time.update();
                            if time.is_expired
                            {
                                time.add_hours(24);
                                let event = Event::new(task.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32);
                                let _ = handler.tick(SchedulerEvent::FinishCycle(event)).await;
                            }
                            else if minutes != 0
                            {
                                let event = Event::new(task.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32);
                                let _ = handler.tick(SchedulerEvent::Tick(event)).await;
                            }
                        }
                    },
                    RepeatingStrategy::Monthly =>
                    {
                        if let Some(time) = task.time.as_mut()
                        {
                            time.update();
                            if time.is_expired
                            {
                                time.add_months(1);
                                let event = Event::new(task.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32);
                                let _ = handler.tick(SchedulerEvent::FinishCycle(event)).await;
                            }
                            else if minutes != 0
                            {
                                let event = Event::new(task.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32);
                                let _ = handler.tick(SchedulerEvent::Tick(event)).await;
                            }
                        }
                    },
                }
            }
            guard.retain(|d| !d.finished);
            logger::debug!("tasks in pool {:?}", &guard);
            drop(guard);
            tokio::time::sleep(tokio::time::Duration::from_millis(60000)).await;
            //reset timer, we not need u32 overflow on long running tasks
            if minutes > 24*60
            {
                minutes = 1;
            } 
            else 
            {
                minutes += 1;
            }
        }

    }
    

    pub async fn add_interval_task(&self, id: T, interval: u32, repeating_strategy: RepeatingStrategy) -> bool
    {
        if let RepeatingStrategy::Forever | RepeatingStrategy::Dialy = repeating_strategy
        {
            logger::debug!("added interval task {:?}", &id);
            let task = Task
            {
                interval: Some(interval),
                time: None,
                repeating_strategy,
                finished: false,
                id
            };
            let mut guard = self.0.write().await;
            guard.push(task);
            return true;
        }
        else 
        {
            logger::error!("В задаче {:?} стратегия повтора должна быть установлена на `once` `dialy` или `forever`, задача выполнена не будет", &id);    
            return false;
        }
    }

    pub async fn add_date_task(&self, id: T, date: Date, repeating_strategy: RepeatingStrategy)
    {
        logger::debug!("added date task {:?}", &id);
        let mut guard = self.0.write().await;
        let task = Task
        {
            interval: None,
            time: Some(Time::new(date)),
            repeating_strategy,
            finished: false,
            id
        };
        guard.push(task);
    }

}


fn time_diff(current_date: &Date, checked_date: &Date) -> i64
{
    checked_date.as_naive_datetime().and_utc().timestamp() - current_date.as_naive_datetime().and_utc().timestamp()
}

#[cfg(test)]
mod tests
{
    use std::{fmt::Debug, hash::Hash, sync::Arc};

    use utilites::Date;
    use crate::SchedulerEvent;

    #[test]
    fn test_interval_current()
    {
        let minutes = 11;
        let interval = 3;
        let d = minutes % interval;
        println!("{}", d);
    }

    ///can add to struct Arc<RwLock<T>> for use in handler 
    struct TestStruct
    {
        
    }
    impl<T> super::SchedulerHandler<T> for TestStruct where T: PartialEq + Eq + Hash + Send + Sync + Debug + ToString + Clone
    {
        fn tick(&self, event: SchedulerEvent<T>) -> impl std::future::Future<Output = ()>
        {
            async 
            {
                match event
                {
                    SchedulerEvent::Tick(t) => 
                    {
                        logger::info!("tick: {:?}", t);
                    },
                    SchedulerEvent::Expired(e) => logger::info!("expired: {:?}", e),
                    SchedulerEvent::Finish(f) => logger::info!("finish: {:?}", f),
                    SchedulerEvent::FinishCycle(fc) => logger::info!("finish_cycle: {:?}", fc),
                };
            }
        }
    }
    
    #[tokio::test]
    async fn test_scheduller()
    {
        let _ = logger::StructLogger::new_default();
        let d1 = Date::now().add_minutes(1);
        let scheduler = super::Scheduler::new();
        let _ = scheduler.add_date_task(Arc::new("123"), d1, crate::RepeatingStrategy::Dialy).await;
        let _ = scheduler.add_interval_task(Arc::new("001"), 1, crate::RepeatingStrategy::Once).await;
        let sch_for_run = scheduler.clone();
        tokio::spawn(async move 
        {
            let test = TestStruct{};
            sch_for_run.run(test).await;
        });
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_millis(63000)).await;
            let _ = scheduler.add_interval_task(Arc::new("002"), 1, crate::RepeatingStrategy::Once).await;
        }
    }
    #[tokio::test]
    async fn test_scheduller2()
    {
        let _ = logger::StructLogger::new_default();
        let d1 = Date::now().sub_minutes(10);
        let scheduler = super::Scheduler::new();
        let _ = scheduler.add_date_task(Arc::new("test_expired_date"), d1, crate::RepeatingStrategy::Dialy).await;
        let sch_for_run = scheduler.clone();
        let test = TestStruct{};
        sch_for_run.run(test).await;
    }

    #[test]
    fn test_interval_1()
    {
        let result = 1%1;
        println!("{}", result);
    }
}