//! This type is used in cron objects to define the cron expression and timeout for a method job.

/// It implements the From trait for strings
#[derive(Debug, Clone, Default)]
pub struct CronFrameExpr {
    seconds: String,
    minutes: String,
    hour: String,
    day_month: String,
    month: String,
    day_week: String,
    year: String,
    timeout: u64,
}

impl CronFrameExpr {
    /// Creates a new CronFrameExpr instance where:
    /// - s   is seconds
    /// - m   is minutes
    /// - h   is hour
    /// - dm  is day_month
    /// - mth is month
    /// - dw  is day_week
    /// - y   is year
    /// - t   is timeout
    ///
    /// ```
    /// use cronframe::CronFrameExpr;
    /// fn main(){
    ///     let my_expr = CronFrameExpr::new("0", "5", "10-14", "*", "*", "Sun", "*", 0);
    /// }
    /// ```
    ///
    pub fn new(s: &str, m: &str, h: &str, dm: &str, mth: &str, dw: &str, y: &str, t: u64) -> Self {
        CronFrameExpr {
            seconds: s.to_string(),
            minutes: m.to_string(),
            hour: h.to_string(),
            day_month: dm.to_string(),
            month: mth.to_string(),
            day_week: dw.to_string(),
            year: y.to_string(),
            timeout: t,
        }
    }

    pub fn expr(&self) -> String {
        format!(
            "{} {} {} {} {} {} {}",
            self.seconds,
            self.minutes,
            self.hour,
            self.day_month,
            self.month,
            self.day_week,
            self.year
        )
    }

    pub fn timeout(&self) -> u64 {
        self.timeout
    }
}

impl From<&str> for CronFrameExpr {
    fn from(item: &str) -> Self {
        let items: Vec<_> = item.split(" ").collect();
        CronFrameExpr::new(
            items[0],
            items[1],
            items[2],
            items[3],
            items[4],
            items[5],
            items[6],
            items[7].parse().unwrap(),
        )
    }
}
