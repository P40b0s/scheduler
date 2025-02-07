use std::{borrow::Cow, fmt::{Debug, Display}, ops::{Deref, DerefMut}, pin::Pin, sync::{Arc, LazyLock}, task::{Context, Poll}, time::Duration};
use serde::{Deserialize, Serialize};
use tokio::{stream, sync::{mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender}, RwLock}};
use tokio_stream::Stream;
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
        let now = Date::now();
        //let current_timestramp = now.as_naive_datetime().and_utc().timestamp();
        let finish_timestramp = time_diff(&now, &new_date);
        self.current_timestramp = 0;
        self.finish_timestramp = finish_timestramp;
        self.is_expired = false;
        self.time = new_date;
    }
    pub fn add_months(&mut self, m: u32)
    {
        if let Some(new_date) =  Date::now().with_time(&self.time).add_months(m)
        {
            let now = Date::now();
            //let current_timestramp = now.as_naive_datetime().and_utc().timestamp();
            let finish_timestramp = time_diff(&now, &new_date);
            self.current_timestramp = 0;
            self.finish_timestramp = finish_timestramp;
            self.is_expired = false;
            self.time = new_date;
        }
    }
}

#[derive(Debug)]
struct Task<'a>
{
    id: Cow<'a, str>,
    ///интервал запуска в минутах
    interval: Option<u32>,
    ///время когда планировщик должен закончить задание
    time: Option<Time>,
    ///Стратегия повтора задачи
    repeating_strategy: RepeatingStrategy,
    finished: bool,
}
#[derive(Debug)]
pub struct Scheduler<'a>
{
    tasks: Arc<RwLock<Vec<Task<'a>>>>,
    sender: Option<Sender<SchedulerEvent<'a>>>,
    receiver: Option<Receiver<SchedulerEvent<'a>>>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum RepeatingStrategy
{
    Once,
    Dialy,
    Forever,
    Monthly,
    // #[cfg(test)]
    // Test
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
pub struct Event<'a> where Self: Send
{
    pub id: Cow<'a, str>,
    pub current: u32,
    pub len: u32
}
impl<'a> Event<'a>
{
    fn new(id: Cow<'a, str>, current: u32, len: u32) -> Self
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
pub enum SchedulerEvent<'a>
{
    ///время указанное в задаче уже прошло а задача должна выполнятся один раз, повторное время для нее назначено не будет
    Expired(Cow<'a, str>),
    Tick(Event<'a>),
    Finish(Cow<'a, str>),
    FinishCycle(Event<'a>)
}



impl<'a> Scheduler<'a>
{
    pub fn new() -> Self
    {
        Self
        {
            tasks: Arc::new(RwLock::new(Vec::new())),
            sender: None,
            receiver: None
        }
    }
    pub async fn get_receiver(&mut self) -> Receiver<SchedulerEvent<'a>>
    {
        let (sender, receiver) = tokio::sync::mpsc::channel::<SchedulerEvent<'a>>(10);
        self.sender = Some(sender);
        receiver
    }

    async fn send_tick(&self, id: Cow<'a, str>, current: u32, len: u32)
    {
        if let Some(sender) = self.sender.as_ref()
        {
            let event = Event::new(id, current, len);
            let _ = sender.send(SchedulerEvent::Tick(event)).await;
        }
    }
    async fn send_cycle_finish(&self, id: Cow<'a, str>, current: u32, len: u32)
    {
        if let Some(sender) = self.sender.as_ref()
        {
            let event = Event::new(id, current, len);
            let _ = sender.send(SchedulerEvent::FinishCycle(event)).await;
        }
    }
    async fn send_finish(&self, id: Cow<'a, str>)
    {
        if let Some(sender) = self.sender.as_ref()
        {
            let _ = sender.send(SchedulerEvent::Finish(id)).await;
        }
    }
    async fn send_expired(&self, id: Cow<'a, str>)
    {
        if let Some(sender) = self.sender.as_ref()
        {
            let _ = sender.send(SchedulerEvent::Expired(id)).await;
        }
    }

    pub async fn run(&self)
    {
        let mut minutes: u32 = 0;
        loop 
        {
            let mut guard = self.tasks.write().await;
            for t in guard.iter_mut()
            {
                if !t.finished
                {
                    match t.repeating_strategy
                    {
                        RepeatingStrategy::Once =>
                        {
                            if let Some(interval) = t.interval.as_ref()
                            {
                                if minutes != 0
                                {
                                    let div = minutes % interval;
                                    if div == 0
                                    {
                                        self.send_finish(t.id.clone()).await;
                                        t.finished = true;
                                    }
                                    else
                                    {
                                        self.send_tick(t.id.clone(), div, *interval).await;
                                    }
                                }
                            }
                            else if let Some(time) = t.time.as_mut()
                            {
                                time.update();
                                if minutes == 0 && time.is_expired
                                {
                                    self.send_expired(t.id.clone()).await;
                                }
                                else if minutes != 0
                                {
                                    if time.is_expired
                                    {
                                        self.send_finish(t.id.clone()).await;
                                        t.finished = true;
                                    }
                                    else
                                    {
                                        self.send_tick(t.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32).await;
                                    }
                                }
                            }
                        },
                        RepeatingStrategy::Dialy | RepeatingStrategy::Forever =>
                        {
                            if let Some(interval) = t.interval.as_ref()
                            {
                                if minutes != 0
                                {
                                    let div = minutes % interval;
                                    if div == 0
                                    {
                                        self.send_cycle_finish(t.id.clone(), div, *interval).await;
                                    }
                                    else
                                    {
                                        self.send_tick(t.id.clone(), div, *interval).await;
                                    }
                                }
                            }
                            else if let Some(time) = t.time.as_mut()
                            {
                                time.update();
                                if time.is_expired
                                {
                                    time.add_hours(24);
                                    self.send_cycle_finish(t.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32).await;
                                }
                                else if minutes != 0
                                {
                                    self.send_tick(t.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32).await;
                                }
                            }
                        },
                        RepeatingStrategy::Monthly =>
                        {
                            if let Some(time) = t.time.as_mut()
                            {
                                time.update();
                                if time.is_expired
                                {
                                    time.add_months(1);
                                    self.send_cycle_finish(t.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32).await;
                                }
                                else if minutes != 0
                                {
                                    self.send_tick(t.id.clone(), time.current_timestramp as u32, time.finish_timestramp as u32).await;
                                }
                            }
                        },
                    }
                }
            }
           
            guard.retain(|d| !d.finished);
            drop(guard);

            tokio::time::sleep(tokio::time::Duration::from_millis(60000)).await;
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
    

    pub async fn add_interval_task(&self, id: Cow<'a, str>, interval: u32, repeating_strategy: RepeatingStrategy)
    {
        let task = Task
        {
            interval: Some(interval),
            time: None,
            repeating_strategy,
            finished: false,
            id
        };
        let mut guard = self.tasks.write().await;
        guard.push(task);
    }
    pub async fn add_error_task(&self, id: Cow<'a, str>)
    {
        let task = Task
        {
            interval: Some(999),
            time: None,
            repeating_strategy: RepeatingStrategy::Once,
            finished: true,
            id
        };
        let mut guard = self.tasks.write().await;
        guard.push(task);
    }

    pub async fn add_date_task<T: Into<Cow<'a, str>>>(&self, id: T, date: Date, repeating_strategy: RepeatingStrategy)
    {
        let mut guard = self.tasks.write().await;
        let task = Task
        {
            interval: None,
            time: Some(Time::new(date)),
            repeating_strategy,
            finished: false,
            id: id.into()
        };
        guard.push(task);
    }

}

trait Test<'a>: Send
{
    fn tick(event: Event<'a>) -> impl std::future::Future<Output = ()> + Send;
    async fn finish_cycle(event: Event<'a>);
    async fn expired(event: Cow<'a, str>);
    async fn finish(event: Cow<'a, str>);
}


fn current_timestramp(date: &Date) -> u32
{
    let now = Date::now();
    let target = time_diff(&now, date);
    target as u32
}
pub fn time_diff(current_date: &Date, checked_date: &Date) -> i64
{
    checked_date.as_naive_datetime().and_utc().timestamp() - current_date.as_naive_datetime().and_utc().timestamp()
}

#[cfg(test)]
mod tests
{
    use std::borrow::Cow;

    use utilites::Date;

    use crate::{Event, SchedulerEvent};

    #[test]
    fn test_interval_current()
    {
        let minutes = 11;
        let interval = 3;
        let d = minutes % interval;
        println!("{}", d);
    }



    #[tokio::test]
    async fn test_scheduller()
    {
        let _ = logger::StructLogger::new_default();
        let d1 = Date::now().add_minutes(1);
        let d2 = Date::now().add_minutes(2);
        let interval = 2;
        let interval2 = 3;
        let interval = 3;
        let mut scheduler = super::Scheduler::new();
        let mut receiver = scheduler.get_receiver().await;
        tokio::spawn(async move 
        {
            while let Some(r) = receiver.recv().await
            {
                match r
                {
                    SchedulerEvent::Tick(t) => logger::info!("tick: {:?}", t),
                    SchedulerEvent::Expired(e) => logger::info!("expired: {}", e),
                    SchedulerEvent::Finish(f) => logger::info!("finish: {}", f),
                    SchedulerEvent::FinishCycle(fc) => logger::info!("finish_cycle: {:?}", fc),
                }
            }
        });
        let _ = scheduler.add_date_task("123", d1, crate::RepeatingStrategy::Dialy).await;
        scheduler.run().await;
    }
}