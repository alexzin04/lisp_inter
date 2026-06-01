#[derive(Default, Debug)]
pub struct Profiler {
    pub constant_folds: usize,
    pub pure_functions_detected: usize,

    pub total_calls: usize,
    pub tail_calls_optimized: usize,
    pub memo_cache_hits: usize,
    pub memo_cache_misses: usize,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            constant_folds: 0,
            pure_functions_detected: 0,
            total_calls: 0,
            tail_calls_optimized: 0,
            memo_cache_hits: 0,
            memo_cache_misses: 0,
        }
    }
    pub fn print_report(&self) {
        println!("\n=== ОТЧЕТ ОПТИМИЗАТОРА ===");
        println!("Свернуто констант:       {}", self.constant_folds);
        println!("Найдено чистых функций:  {}", self.pure_functions_detected);
        println!("--------------------------");
        println!("Всего вызовов:           {}", self.total_calls);
        println!("Хвостовых вызовов (TCO): {}", self.tail_calls_optimized);
        println!("Попаданий в кэш (Memo):  {}", self.memo_cache_hits);
        println!("Промахов кэша (Miss):    {}", self.memo_cache_misses);
        if self.memo_cache_hits + self.memo_cache_misses > 0 {
            let ratio = (self.memo_cache_hits as f64
                / (self.memo_cache_hits + self.memo_cache_misses) as f64)
                * 100.0;
            println!("Эффективность кэша:      {:.2}%", ratio);
        }
        println!("==========================\n");
    }
}
