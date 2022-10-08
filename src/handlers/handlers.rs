use crate::stats::{Loadavg, MemoryWrapper, StatsResponse};
use crate::Stats;
use actix_web::body::BoxBody;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::{get, Error, HttpResponse, Responder};
use actix_web_lab::__reexports::serde_json;
use askama::Template;
use core::fmt;
use minify::html::minify;

extern crate minify;

extern crate systemstat;
use std::thread;
use std::time::Duration;
use systemstat::platform::PlatformImpl;
use systemstat::{ByteSize, LoadAverage, Memory, Platform, PlatformMemory, System};

impl fmt::Display for Loadavg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Load avg: ")
    }
}

impl fmt::Display for MemoryWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mem ")
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "index.html")] // using the template in this path, relative
                                 // to the `templates` dir in the crate root
struct StatsTemplate {
    stats: Stats,
}

impl Responder for StatsResponse {
    type Body = BoxBody;
    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        let res_body = serde_json::to_string(&self).unwrap();

        // Create HttpResponse and set Content Type
        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(res_body)
    }
}

pub async fn get_stats_from_linux(sys: PlatformImpl) -> Stats {
    match sys.cpu_load_aggregate() {
        Ok(cpu) => {
            println!("\nMeasuring CPU load...");
            thread::sleep(Duration::from_secs(1));
            let cpu = cpu.done().unwrap();

            let load_avg = match sys.load_average() {
                Ok(load) => Loadavg {
                    one: load.one,
                    five: load.five,
                    fifteen: load.fifteen,
                },
                Err(_) => Loadavg {
                    one: 0.0,
                    five: 0.0,
                    fifteen: 0.0,
                },
            };

            let memory_details = match sys.memory() {
                Ok(mem) => mem,
                Err(err) => panic!("Error: {:?}", err),
            };

            println!(
                "CPU load: {}% user, {}% nice, {}% system, {}% intr, {}% idle ",
                cpu.user * 100.0,
                cpu.nice * 100.0, // TG :cpu load nice
                cpu.system * 100.0,
                cpu.interrupt * 100.0,
                cpu.idle * 100.0
            );

            let cpu_usage = format!(
                "CPU load: {}% user, {}% nice, {}% system, {}% intr, {}% idle ",
                cpu.user * 100.0,
                cpu.nice * 100.0, // TG :cpu load nice
                cpu.system * 100.0,
                cpu.interrupt * 100.0,
                cpu.idle * 100.0
            );
            // memory_details
            let stats = Stats {
                loadavg: load_avg,
                cpu_usage,
                memory_usage: MemoryWrapper {
                    memory_usage: memory_details,
                },
            };

            return stats;
        }
        Err(x) => println!("\nCPU load: error: {}", x),
    }
    let av = LoadAverage {
        one: 0.0,
        five: 0.0,
        fifteen: 0.0,
    };
    let b: ByteSize = ByteSize::mb(0);

    let mem_usage = MemoryWrapper {
        memory_usage: Memory {
            total: b,
            free: b,
            platform_memory: PlatformMemory {
                total: b,
                active: b,
                inactive: b,
                wired: b,
                free: b,
                purgeable: b,
                speculative: b,
                compressor: b,
                throttled: b,
                external: b,
                internal: b,
                uncompressed_in_compressor: b,
            },
        },
    };
    Stats {
        loadavg: Loadavg {
            one: 0.0,
            five: 0.0,
            fifteen: 0.0,
        },
        cpu_usage: "Error".to_string(),
        memory_usage: mem_usage,
    }
}

#[get("")]
pub async fn index_page() -> Result<HttpResponse, Error> {
    let sys = System::new();

    match sys.cpu_temp() {
        Ok(cpu_temp) => println!("\nCPU temp: {}", cpu_temp),
        Err(x) => println!("\nCPU temp: {}", x),
    }

    let stats: Stats = get_stats_from_linux(sys).await;
    println!("{:?}", stats);

    let stats_html = StatsTemplate { stats }; // instantiate your struct

    let html_str = minify(&stats_html.render().unwrap());

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(html_str))
}

#[get("/v1/stats")]
async fn status_get_api() -> impl Responder {
    let sys = System::new();
    let stats: Stats = get_stats_from_linux(sys).await;
    println!("{:?}", stats);

    let has_error = stats.cpu_usage == "Error";

    let stats_response = StatsResponse {
        result: !has_error,
        data: stats,
    };

    let response = serde_json::to_string(&stats_response).unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(response)
}
