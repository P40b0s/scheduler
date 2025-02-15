mod event;
mod handler;
mod task;
use task::Task;
mod time;
use time::Time;
use std::{fmt::Debug,  sync::Arc};
use tokio::sync::RwLock;
use utilites::Date;
pub use handler::SchedulerHandler;
pub use event::{SchedulerEvent, RepeatingStrategy, Event};



///clone only cloning internal ref Arc<RwLock<Vec<Task>>>
#[derive(Debug)]
pub struct Scheduler<T>(Arc<RwLock<Vec<Task<T>>>>) where Self: Send + Sync, T: PartialEq + Eq + Send + Clone;

impl<T> Clone for Scheduler<T> where T: PartialEq + Eq + Send + Sync + Clone
{
    fn clone(&self) -> Self 
    {
        Self(self.0.clone())
    }
}

impl<T> Scheduler<T> where T: PartialEq + Eq + Send + Sync + Clone + Debug
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
                if !interval_worker(task, &mut minutes, &handler).await
                {
                    date_worker(task, &mut minutes, &handler).await;
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
        if let RepeatingStrategy::Forever | RepeatingStrategy::Dialy | RepeatingStrategy::Once = repeating_strategy
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

    pub async fn add_date_task(&self, id: T, date: Date, repeating_strategy: RepeatingStrategy) -> bool
    {
        logger::debug!("added date task {:?}", &id);
        let mut guard = self.0.write().await;
        let error = format!("В задаче {:?} со стратегией повтора `once` дата {} должна быть больше текущей {}", &id, date.to_string(), Date::now());
        let time = Time::new(date, true);
        let expired = time.is_expired;
        let add_task = ||
        {
            let task = Task
            {
                interval: None,
                time: Some(time),
                repeating_strategy,
                finished: false,
                id
            };
            guard.push(task);
        };
        if let RepeatingStrategy::Once = repeating_strategy
        {
            if expired
            {
               
                logger::error!("{}", error);
                return false; 
            }
            else
            {
                add_task();
                return true;
            }
        }
        else 
        {
            add_task();
            return true;
        }
       
    }

}


async fn interval_worker<T, H: SchedulerHandler<T>>(task: &mut Task<T>, minutes: &mut u32, handler: &H) -> bool
where T: PartialEq + Eq + Send + Sync + Clone + Debug
{
    if let Some(interval) = task.interval.as_ref()
    {
        if minutes != &0
        {
            let div = *minutes % interval;
            if div == 0
            {
                if let RepeatingStrategy::Once = task.repeating_strategy
                {
                    handler.tick(SchedulerEvent::Finish(task.id.clone())).await;
                    task.finished = true;
                }
                else 
                {
                    let event = Event::new(task.id.clone(), div, *interval);
                    let _ = handler.tick(SchedulerEvent::FinishCycle(event)).await;
                }
            }
            else
            {
                let event = Event::new(task.id.clone(), div, *interval);
                let _ = handler.tick(SchedulerEvent::Tick(event)).await;
            }
        }
        return true;
    }
    false
}

async fn date_worker<T, H: SchedulerHandler<T>>(task: &mut Task<T>, minutes: &mut u32, handler: &H)
where T: PartialEq + Eq + Send + Sync + Clone + Debug
{
    if let Some(time) = task.time.as_mut()
    {
        time.update();
        let mut need_tick_msg = false;
        match task.repeating_strategy
        {
            RepeatingStrategy::Once =>
            {
                if time.is_expired
                {
                    handler.tick(SchedulerEvent::Finish(task.id.clone())).await;
                    task.finished = true;
                }
            },
            _ =>
            {
                if time.is_expired
                {
                    if time.is_new
                    {
                        add_time(time, task.repeating_strategy);
                        need_tick_msg = true;
                    }
                    else 
                    {
                        add_time(time, task.repeating_strategy);
                        let event = Event::new(task.id.clone(), time.current as u32, time.finish as u32);
                        let _ = handler.tick(SchedulerEvent::FinishCycle(event)).await;
                    }
                }
                else 
                {
                    need_tick_msg = true;
                }
            }
        }
        if minutes != &0 && need_tick_msg
        {
            let event = Event::new(task.id.clone(), time.current as u32, time.finish as u32);
            let _ = handler.tick(SchedulerEvent::Tick(event)).await;
        }
    }
}
fn add_time(time: &mut Time, repeating_strategy: RepeatingStrategy)
{
    match repeating_strategy
    {
        RepeatingStrategy::Dialy | RepeatingStrategy::Forever =>
        {
            time.add_hours(24);
        },
        RepeatingStrategy::Monthly =>
        {
            time.add_months(1);
        },
        _ => ()
    };
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
        let _ = scheduler.add_date_task(Arc::new("test_expired_date"), d1, crate::RepeatingStrategy::Monthly).await;
        let sch_for_run = scheduler.clone();
        tokio::spawn(async move 
        {
            let test = TestStruct{};
            sch_for_run.run(test).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(63000)).await;
        let d1 = Date::now().sub_minutes(4000);
        let _ = scheduler.add_date_task(Arc::new("test_added_expired_date"), d1, crate::RepeatingStrategy::Monthly).await;
        loop 
        {
            tokio::time::sleep(tokio::time::Duration::from_millis(63000)).await;
           
        }
       
    }

    #[test]
    fn test_interval_1()
    {
        let result = 1%1;
        println!("{}", result);
    }
}