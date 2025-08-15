use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use futures::{channel::oneshot, FutureExt};
use pyo3::{ffi::c_str, prelude::*, IntoPyObject, Python};
use serde::Serialize;

const N: usize = 100;
const K: f32 = 8. * PI / N as f32;

#[derive(Serialize, Debug, IntoPyObject, Clone)]
struct Sample {
    fut_name: String,
    value: f32,
    start: u128,
    end: u128,
    thread_id: usize,
}

fn sin(x: f32) -> f32 {
    std::thread::sleep(Duration::from_micros(100));
    x.sin()
}

fn sin_heavy(x: f32) -> f32 {
    std::thread::sleep(Duration::from_micros(500));
    x.sin()
}

async fn produce_sin(run_start: Instant, fut_name: impl ToString) -> Vec<Sample> {
    let mut samples = Vec::new();

    for i in 0..N {
        let start = run_start.elapsed().as_micros();
        let value = sin(i as f32 * K);
        let end = run_start.elapsed().as_micros();

        samples.push(Sample {
            fut_name: fut_name.to_string(),
            value,
            start,
            end,
            thread_id: thread_id::get(),
        });

        tokio::task::yield_now().await;
    }

    samples
}

async fn produce_sin_heavy(run_start: Instant, fut_name: impl ToString) -> Vec<Sample> {
    let mut samples = Vec::new();

    for i in 0..N {
        let start = run_start.elapsed().as_micros();
        let value = sin_heavy(i as f32 * K);
        let end = run_start.elapsed().as_micros();

        samples.push(Sample {
            fut_name: fut_name.to_string(),
            value,
            start,
            end,
            thread_id: thread_id::get(),
        });

        tokio::task::yield_now().await;
    }

    samples
}

async fn produce_sin_heavy_blocking(run_start: Instant, fut_name: impl ToString) -> Vec<Sample> {
    let mut samples = Vec::new();

    for i in 0..N {
        let start = run_start.elapsed().as_micros();

        let (t_id, value) = tokio::task::spawn_blocking(move || {
            let value = sin_heavy(i as f32 * K);
            let t_id = thread_id::get();

            (t_id, value)
        })
        .await
        .unwrap();

        let end = run_start.elapsed().as_micros();

        samples.push(Sample {
            fut_name: fut_name.to_string(),
            value,
            start,
            end,
            thread_id: t_id,
        });

        tokio::task::yield_now().await;
    }

    samples
}

fn plot_samples(
    samples: Vec<Sample>,
    include_times: bool,
    output_filename: &str,
    zoom: f32,
) -> Result<(), pyo3::PyErr> {
    println!("Plotting {} samples to {output_filename}", samples.len());
    let code = c_str!(include_str!("./py/plot.py"));

    Python::with_gil(|py| -> PyResult<()> {
        let module = PyModule::from_code(py, code, c_str!("plot.py"), c_str!("plot"))?;
        let plot_fn = module.getattr("plot")?;

        plot_fn.call1((samples, include_times, zoom, output_filename))?;

        Ok(())
    })
}

async fn two_futures() -> Vec<Sample> {
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0").boxed());
    futs.push(produce_sin(run_start, "fut1").boxed());

    let samples = futures::future::join_all(futs).await;
    let samples = samples.into_iter().flatten().collect::<Vec<_>>();

    samples
}

async fn cpu_intensive() -> Vec<Sample> {
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0").boxed());
    futs.push(produce_sin(run_start, "fut1").boxed());
    futs.push(produce_sin_heavy(run_start, "high cpu").boxed());

    let samples = futures::future::join_all(futs).await;

    samples.into_iter().flatten().collect::<Vec<_>>()
}

async fn spawn_task() -> Vec<Sample> {
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0").boxed());
    futs.push(produce_sin(run_start, "fut1").boxed());

    futs.push(
        tokio::spawn(produce_sin_heavy(run_start, "spawned").boxed())
            .map(|res| res.unwrap())
            .boxed(),
    );

    let samples = futures::future::join_all(futs).await;
    let samples = samples.into_iter().flatten().collect::<Vec<_>>();

    samples
}

async fn many_spawn_task() -> Vec<Sample> {
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0").boxed());

    for i in 1..7 {
        futs.push(
            tokio::spawn(produce_sin_heavy(run_start, format!("spawned{i}")))
                .map(|res| res.unwrap())
                .boxed(),
        );
    }

    let samples = futures::future::join_all(futs).await;
    let samples = samples.into_iter().flatten().collect::<Vec<_>>();

    samples
}

async fn many_spawn_blocking() -> Vec<Sample> {
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0").boxed());

    for i in 1..7 {
        futs.push(produce_sin_heavy_blocking(run_start, format!("spawn_\nblocking{i}")).boxed());
    }

    let samples = futures::future::join_all(futs).await;
    let samples = samples.into_iter().flatten().collect::<Vec<_>>();

    samples
}

async fn rayon_sin_heavy(i: f32) -> f32 {
    let (tx, rx) = oneshot::channel();

    rayon::spawn(move || {
        tx.send(sin_heavy(i)).expect("Failed to send result");
    });

    rx.await.expect("Failed to receive result")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pyo3::prepare_freethreaded_python();

    let two_futures_samples = two_futures().await;
    plot_samples(
        two_futures_samples.clone(),
        false,
        "resources/two_futures.png",
        1.,
    )?;
    plot_samples(
        two_futures_samples.clone(),
        false,
        "resources/two_futures.png",
        1.,
    )?;
    plot_samples(
        two_futures_samples.clone(),
        true,
        "resources/two_futures_with_times.png",
        1.,
    )?;
    plot_samples(
        two_futures_samples,
        true,
        "resources/two_futures_zoom.png",
        0.25,
    )?;

    let spawn_task_samples = spawn_task().await;
    plot_samples(
        spawn_task_samples.clone(),
        true,
        "resources/spawn_task.png",
        1.,
    )?;
    plot_samples(
        spawn_task_samples,
        true,
        "resources/spawn_task_zoom.png",
        0.25,
    )?;

    let many_spawn_task_samples = many_spawn_task().await;
    plot_samples(
        many_spawn_task_samples.clone(),
        true,
        "resources/many_spawn_task.png",
        1.,
    )?;
    plot_samples(
        many_spawn_task_samples,
        true,
        "resources/many_spawn_task_zoom.png",
        0.25,
    )?;

    let cpu_intensive_samples = cpu_intensive().await;
    plot_samples(
        cpu_intensive_samples.clone(),
        true,
        "resources/cpu_intensive.png",
        1.,
    )?;
    plot_samples(
        cpu_intensive_samples,
        true,
        "resources/cpu_intensive_zoom.png",
        0.25,
    )?;

    let many_spawn_blocking_samples = many_spawn_blocking().await;
    plot_samples(
        many_spawn_blocking_samples.clone(),
        true,
        "resources/many_spawn_blocking.png",
        1.,
    )?;
    plot_samples(
        many_spawn_blocking_samples,
        true,
        "resources/many_spawn_blocking_zoom.png",
        0.25,
    )?;

    Ok(())
}
