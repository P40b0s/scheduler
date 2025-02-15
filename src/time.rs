use utilites::Date;

#[derive(Debug)]
pub (crate) struct Time
{
    pub (crate) time: Date,
    pub (crate) current: i64,
    pub (crate) finish: i64,
    pub (crate) is_expired: bool,
    pub (crate) is_new: bool,

}

impl Time
{
    pub fn new(date: Date, is_new: bool) -> Self
    {
        let now = Date::now();
        let finish = &date - &now;
        let current = 0;
        //let finish = date.add_seconds(diff).as_naive_datetime().and_utc().timestamp();
        //let finish = time_diff(&now, &date);
        logger::debug!("current_ts: {} finish_ts: {}", current, finish);
        let is_expired = if finish.is_negative() { true } else { false };
        Self
        {
            time: date,
            current,
            finish,
            is_expired,
            is_new
        }
    }
    pub fn update(&mut self)
    {
        let now = Date::now();
        let diff = &self.time - &now;
        if diff.is_positive()
        {
            self.current = self.finish - diff;
            logger::debug!("UPDATE_TIME diff:{} cur: {}, fin: {}", diff, self.current, self.finish);
        }
        else 
        {
            self.is_expired = true;  
        }
    }
    pub fn add_hours(&mut self, h: i64)
    {
        let new_date =  Date::now().with_time(&self.time).add_minutes(h*60);
        *self = Time::new(new_date, false);
    }
    pub fn add_months(&mut self, m: u32)
    {
        if let Some(new_date) =  Date::now().with_time(&self.time).add_months(m)
        {
            *self = Time::new(new_date, false);
        }
    }
}




// use utilites::Date;

// #[derive(Debug)]
// pub (crate) struct Time
// {
//     pub (crate) time: Date,
//     pub (crate) current: i64,
//     pub (crate) finish: i64,
//     pub (crate) is_expired: bool,

// }

// impl Time
// {
//     pub fn new(date: Date) -> Self
//     {
//         let now = Date::now();
//         let finish = &date - &now;
//         let current = 0;
//         //let finish = date.add_seconds(diff).as_naive_datetime().and_utc().timestamp();
//         //let finish = time_diff(&now, &date);
//         logger::debug!("current_ts: {} finish_ts: {}", current, finish);
//         let is_expired = if finish.is_negative() { true } else { false };
//         Self
//         {
//             time: date,
//             current,
//             finish,
//             is_expired
//         }
//     }
//     pub fn update(&mut self)
//     {
//         let now = Date::now();
//         let diff = time_diff(&now, &self.time);
//         logger::debug!("UPDATE_TIME diff:{} cur: {}, fin: {}", diff, self.current_timestramp, self.finish_timestramp);
//         if diff.is_positive()
//         {
//             self.current_timestramp = self.finish_timestramp - diff;
//         }
//         else 
//         {
//             self.is_expired = true;  
//         }
//     }
//     pub fn add_hours(&mut self, h: i64)
//     {
//         let new_date =  Date::now().with_time(&self.time).add_minutes(h*60);
//         //if we take date now, nned correction with await timer (1 min)
//         let old_date = Date::now().with_time(&self.time);
//         //let current_timestramp = now.as_naive_datetime().and_utc().timestamp();
//         let finish_timestramp = time_diff(&old_date, &new_date);
//         self.current_timestramp = 0;
//         self.finish_timestramp = finish_timestramp;
//         self.is_expired = false;
//         self.time = new_date;
//     }
//     pub fn add_months(&mut self, m: u32)
//     {
//         if let Some(new_date) =  Date::now().with_time(&self.time).add_months(m)
//         {
//             let old_date = Date::now().with_time(&self.time);
//             let finish_timestramp = time_diff(&old_date, &new_date);
//             self.current_timestramp = 0;
//             self.finish_timestramp = finish_timestramp;
//             self.is_expired = false;
//             self.time = new_date;
//         }
//     }
// }
// fn time_diff(current_date: &Date, checked_date: &Date) -> i64
// {
//     checked_date.as_naive_datetime().and_utc().timestamp() - current_date.as_naive_datetime().and_utc().timestamp()
// }
