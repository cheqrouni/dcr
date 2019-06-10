#[macro_use]
extern crate log;
extern crate actix_web;
extern crate env_logger;

use actix_web::error::ErrorBadRequest;
use actix_web::http::{header, Method, StatusCode};
use actix_web::{
    dev, error, guard, middleware, web, App, Error, FromRequest, HttpRequest, HttpResponse,
    HttpServer, Result,
};
use futures::{Future, Stream};
use rand;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, io};

const DCR_VERSION: &str = "0.1";
static HEALTH: AtomicBool = AtomicBool::new(true);

fn debug_handler(body: web::Payload) -> impl Future<Item = HttpResponse, Error = Error> {
    debug!("debug endpoint - entering...");
    body.map_err(Error::from)
        .fold(web::BytesMut::new(), move |mut body, chunk| {
            body.extend_from_slice(&chunk);
            debug!("debug endpoint - fold closure...");
            Ok::<_, Error>(body)
        })
        .and_then(|body| {
            debug!("debug endpoint - and_then");
            info!("{:?}", body);
            //Ok(HttpResponse::Ok().finish())
            Ok(HttpResponse::build(StatusCode::OK)
                .content_type("text/html; charset=utf-8")
                .body("data ingested"))
        })
}


fn main_handler(req: HttpRequest) -> HttpResponse {
    info!(
        "{:#?} {} {} - 200 OK",
        req.version(),
        req.method(),
        req.uri()
    );
    let mut body = String::from("<html><body>");
    body.push_str("<H1>Program</H1>");

    body.push_str(&"<H1>Request</H1>");
    body.push_str("<textarea cols=150 readonly>");
    body.push_str(&format!("Protocol: {:?}\n", req.version()));
    body.push_str(&format!("Method: {:?}\n", req.method()));
    body.push_str(&format!("URI: {:?}", req.uri()));
    body.push_str("</textarea>");

    body.push_str("<H1>Headers</H1>");
    body.push_str("<textarea cols=150 rows=20 readonly>");
    for (key, value) in req.headers() {
        body.push_str(&format!("{}: {:#?}\n", key, value));
    }
    body.push_str("</textarea>");

    body.push_str("<H1>Data</H1>");


    body.push_str("<H1>Env</H1>");
    body.push_str("<hr><textarea cols=150 rows=20 readonly>");
    for (key, value) in env::vars() {
        body.push_str(&format!("{}: {}\n", key, value));
    }
    body.push_str("</textarea>");

    body.push_str("</body></html>\n");

    HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(body)
}

fn health_handler(_req: HttpRequest) -> HttpResponse {

    if HEALTH.load(Ordering::Relaxed) {
        HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body("OK")
    } else {
        HttpResponse::build(StatusCode::SERVICE_UNAVAILABLE)
            .content_type("text/html; charset=utf-8")
            .body("KO")
    }

}

fn health_toggle_handler(req: HttpRequest) -> HttpResponse {

    let hc = !HEALTH.load(Ordering::Relaxed);
    HEALTH.store(hc, Ordering::Relaxed);
    info!(
        "{:#?} {} {} - 200 OK - Healthcheck status toggled to: {} ",
        req.version(),
        req.method(),
        req.uri(),
        hc
    );
    HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(format!("healthcheck toggled to {} state", hc))
}

//nts: add stamp here
fn version_handler(req: HttpRequest, dcr_stamp: String) -> HttpResponse {
    info!(
        "{:#?} {} {} - 200 OK",
        req.version(),
        req.method(),
        req.uri()
    );
    HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(format!("{}", DCR_VERSION))
}

//nts: get input
fn logger_handler(body: web::Payload) -> impl Future<Item = HttpResponse, Error = Error> {
    body.map_err(Error::from)
        .fold(web::BytesMut::new(), move |mut body, chunk| {
            body.extend_from_slice(&chunk);
            Ok::<_, Error>(body)
        })
        .and_then(|body| {
            //let mut output = String::from(format!("{:?}", body));

            //Ok(HttpResponse::Ok().finish())
            Ok(HttpResponse::build(StatusCode::OK)
                .content_type("text/html; charset=utf-8")
                .body("data ingested"))
        })
}


/// 404 handler
fn p404(req: HttpRequest) -> HttpResponse {
    info!(
        "{:#?} {} {} - 404 NOT FOUND",
        req.version(),
        req.method(),
        req.uri()
    );
    HttpResponse::build(StatusCode::NOT_FOUND)
        .content_type("text/html; charset=utf-8")
        .body("NOT FOUND")
}


fn main() -> io::Result<()> {
    // logger init
    env::set_var("RUST_LOG", "dcr=debug");
    env_logger::init();

    // parse env
    //let dcr_basepath = env::var("DCR_BASEPATH").expect("DCR_BASEPATH must be set");

    let dcr_basepath = "/dcr";
    let dcr_port = env::var("DCR_PORT").expect("DCR_PORT must be set");
    let dcr_stamp = env::var("DCR_STAMP").expect("DCR_STAMP must be set");
    let dcr_healthcheck = env::var("DCR_HEALTH").expect("DCR_HEALTH must be set");
    let dcr_logger = env::var("DCR_LOGGER").expect("DCR_LOGGER must be set");

    info!("Config: version {}{} on port {} and path {}. Inital health answer is {} and logger endpoint is {}",DCR_VERSION, dcr_stamp, dcr_port, dcr_basepath, HEALTH.load(Ordering::Relaxed), dcr_logger);

    let sys = actix_rt::System::new("dcr");

    HEALTH.store(true, Ordering::Relaxed);

    let path_health = format!("{}/health", dcr_basepath);
    let path_version = format!("{}/version", dcr_basepath);
    let path_logger = format!("{}/logger", dcr_basepath);

    // server
    HttpServer::new(move || {
        App::new()
            // enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())
            .service(
                web::resource(&path_health)
                    .route(web::get().to(health_handler))
                    .route(web::put().to(health_toggle_handler))
                    .route(web::post().to(health_toggle_handler)),
            )
            .service(
                web::resource(&path_logger)
                    .route(web::put().to_async(logger_handler))
                    .route(web::post().to_async(logger_handler)),
            )
            .service(web::resource(&path_version).route(web::get().to(version_handler)))
            .service(web::resource("/debug").route(web::route().to_async(debug_handler)))
            .service(web::resource(&dcr_basepath).route(web::route().to(main_handler)))
            // default
            .default_service(
                // 404 for GET request
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(|| HttpResponse::MethodNotAllowed()),
                    ),
            )
    })
    .bind(format!("127.0.0.1:{}", dcr_port))?
    .start();

    info!(
        "HTTP server successfully started on http://127.0.0.1:{}/{}",
        dcr_port, dcr_basepath
    );
    sys.run()
}

