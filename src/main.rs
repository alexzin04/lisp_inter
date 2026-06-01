mod data;
mod enviroment;
mod function;
mod parcer;
mod profiler;

use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{
    enviroment::RuntimeEnv,
    function::{add_all_basic_func, eval},
};

struct RunOptions {
    memoization_enabled: bool,
    paths: Vec<PathBuf>,
}

fn main() {
    let options = parse_args();

    let files = match lisp_files("test", &options.paths) {
        Ok(files) => files,
        Err(err) => {
            eprintln!("Не удалось открыть примеры: {err}");
            return;
        }
    };

    if files.is_empty() {
        println!("В папке test нет .lisp файлов");
        return;
    }

    println!("Найдено примеров: {}", files.len());
    println!(
        "Memoization: {}",
        if options.memoization_enabled {
            "включена"
        } else {
            "выключена"
        }
    );
    println!();

    let suite_start = Instant::now();
    let mut passed = 0usize;
    let mut failed = 0usize;

    for path in files {
        if run_lisp_file(&path, options.memoization_enabled) {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    println!("=== ИТОГО ===");
    println!("Успешно: {}", passed);
    println!("С ошибкой: {}", failed);
    println!("Общее время: {}", format_duration(suite_start.elapsed()));
}

fn parse_args() -> RunOptions {
    let mut memoization_enabled = true;
    let mut paths = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--no-memo" | "--no-memoization" => memoization_enabled = false,
            "--memo" | "--memoization" => memoization_enabled = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => paths.push(PathBuf::from(arg)),
        }
    }

    RunOptions {
        memoization_enabled,
        paths,
    }
}

fn print_usage() {
    println!("Usage:");
    println!("  cargo run");
    println!("  cargo run -- --no-memo");
    println!("  cargo run -- --no-memo test/memoization.lisp");
    println!();
    println!("Flags:");
    println!("  --no-memo, --no-memoization  Disable memoization for pure functions");
    println!("  --memo, --memoization        Enable memoization explicitly");
}

fn lisp_files(dir: impl AsRef<Path>, requested_paths: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !requested_paths.is_empty() {
        for path in requested_paths {
            if path.is_dir() {
                files.extend(lisp_files(path, &[])?);
            } else {
                files.push(path.clone());
            }
        }

        files.sort();
        return Ok(files);
    }

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "lisp") {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn run_lisp_file(path: &Path, memoization_enabled: bool) -> bool {
    println!("=== {} ===", path.display());

    let load_start = Instant::now();
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            println!("Ошибка чтения: {err}");
            println!();
            return false;
        }
    };
    let load_time = load_start.elapsed();

    let parse_start = Instant::now();
    let expressions = match parcer::read_str(&source) {
        Ok(expressions) => expressions,
        Err(err) => {
            println!("Ошибка парсинга: {err}");
            println!("Чтение файла: {}", format_duration(load_time));
            println!();
            return false;
        }
    };
    let parse_time = parse_start.elapsed();

    let mut env = RuntimeEnv::new_global();
    env.borrow_mut().memoization_enabled = memoization_enabled;
    add_all_basic_func(&mut env);

    let mut optimize_time = Duration::ZERO;
    let mut eval_time = Duration::ZERO;
    let mut ok = true;
    let expression_count = expressions.len();

    for (index, expression) in expressions.into_iter().enumerate() {
        let original_view = expression.to_string();

        let optimize_start = Instant::now();
        let optimized = expression.optimize();
        optimize_time += optimize_start.elapsed();

        let optimized_view = optimized.to_string();
        if original_view != optimized_view {
            println!("    optimized: {} -> {}", original_view, optimized_view);
        }

        let eval_start = Instant::now();
        match eval(optimized, &env) {
            Ok(result) => {
                let elapsed = eval_start.elapsed();
                eval_time += elapsed;
                println!(
                    "#{:<2} => {:<30} eval {}",
                    index + 1,
                    result,
                    format_duration(elapsed)
                );
            }
            Err(err) => {
                eval_time += eval_start.elapsed();
                println!("#{} ошибка выполнения: {}", index + 1, err);
                ok = false;
                break;
            }
        }
    }

    println!("Выражений: {}", expression_count);
    println!("Чтение файла: {}", format_duration(load_time));
    println!("Парсинг: {}", format_duration(parse_time));
    println!("Оптимизация: {}", format_duration(optimize_time));
    println!("Выполнение: {}", format_duration(eval_time));
    env.borrow().profiler.borrow().print_report();

    ok
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{:.3} s", duration.as_secs_f64())
    } else if duration.as_millis() > 0 {
        format!("{:.3} ms", duration.as_secs_f64() * 1_000.0)
    } else if duration.as_micros() > 0 {
        format!("{} us", duration.as_micros())
    } else {
        format!("{} ns", duration.as_nanos())
    }
}
