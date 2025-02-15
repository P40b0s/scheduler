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
            logger::debug!("update time: diff:{} cur: {}, fin: {}", diff, self.current, self.finish);
        }
        else 
        {
            self.is_expired = true;  
        }
    }
    pub fn add_hours(&mut self, h: i64)
    {
        let new_date =  Date::now().with_time(&self.time).add_minutes(h*60);
        logger::debug!("set new dialy date: {}", &new_date.to_string());
        *self = Time::new(new_date, false);
    }
    pub fn add_months(&mut self, m: u32)
    {
        if let Some(new_date) =  Date::now().with_time(&self.time).add_months(m)
        {
            logger::debug!("set new monthly date: {}", &new_date.to_string());
            *self = Time::new(new_date, false);
        }
    }
}