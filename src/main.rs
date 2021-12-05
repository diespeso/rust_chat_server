use actix_web::{Result, Error, HttpServer, HttpRequest, HttpResponse, App, web, get, post};
use actix_files::NamedFile;

use actix::{Actor, StreamHandler,
    Addr, AsyncContext, Message, Recipient,
    Context, WrapFuture, ActorFuture, ContextFutureSpawner, fut};
use actix_web_actors::ws;

use std::path::PathBuf;

use handlebars::Handlebars;
use serde_json::json;
use serde::Deserialize;

use rand::Rng;

use std::sync::{Arc, Mutex};
use std::thread;
use std::net::TcpListener;

mod chat;

pub use crate::chat::*;

static IP: &str = "127.0.0.1";
static PORT: u16 = 8080;

#[get("/")]
async fn index(req: HttpRequest) -> Result<NamedFile> {
    let path: PathBuf = "./public/html/index.html".parse().expect("Path no generado");
    Ok(NamedFile::open(path)?)
}

//getting info from the path
#[get("/saludar/{nombre}")]
async fn saludar(info: web::Path<(String,)>) -> HttpResponse {
    let mut hb = Handlebars::new();
    hb.register_template_file("saludar", "./public/html/saludar.hbs").expect("Fallo al registrar template");
    let body = hb.render("saludar", &json!(
        {"name": info.into_inner().0}
    )).expect("Fallo en renderizado de template");
    HttpResponse::Ok().body(body)
}

#[get("/pedir_rfc")]
async fn pedir_rfc(req: HttpRequest) -> Result<NamedFile> {
    let path: PathBuf = "./public/html/pedir_rfc.html".parse().expect("Path no generado");
    Ok(NamedFile::open(path)?)
}

#[derive(Deserialize)]
struct FormRfc {
    ap_pat: String,
    ap_mat: String,
    nombre: String,
    nacimiento: String
}

#[post("/mostrar_rfc")]
async fn mostrar_rfc(info: web::Form<FormRfc>) -> HttpResponse {
    let mut hb = Handlebars::new();
    hb.register_template_file("mostrar_rfc", "./public/html/mostrar_rfc.hbs").expect("Fallo en registrar template");
    
    let info = info.into_inner();
    let mut rfc = info.ap_pat.as_str()[..2].to_owned().to_uppercase();

    rfc.push_str(&info.ap_mat.as_str()[..1].to_owned().to_uppercase());
    rfc.push_str(&info.nombre.as_str()[..1].to_owned().to_uppercase());

    let fecha: Vec<&str> = info.nacimiento.split('-').collect();
    rfc.push_str(fecha[2]);
    rfc.push_str(fecha[1]);
    rfc.push_str(&fecha[0][2..]);

    let mut rng = rand::thread_rng();
    rfc.push(rng.gen_range(b'A'..b'Z') as u8 as char);
    rfc.push(rng.gen_range(b'A'..b'Z') as u8 as char);
    rfc.push_str(&rng.gen_range(0..9).to_string());
 
    let body = hb.render("mostrar_rfc", &json!({"rfc": rfc})).expect("Fallo al renderizar template");
    println!("{:?}", fecha);
    HttpResponse::Ok().body(body)
}

#[get("/css/{archivo}.css")]
async fn serve_css(path: web::Path<String>) -> Result<NamedFile> {
    println!("{}", path.clone());
    Ok(NamedFile::open(
        format!("./public/css/{}.css", path.into_inner())
    )?)
}

#[get("/script/{archivo}.js")]
async fn serve_js(path: web::Path<String>) -> Result<NamedFile> {
    println!("{}", path.clone());
    Ok(NamedFile::open(
        format!("./public/script/{}.js", path.into_inner())
    )?)
}

#[derive(Debug)]
struct ComSocket;

impl ComSocket {
    pub fn new() -> Self {
        Self
    }
}

impl Actor for ComSocket {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ComSocket {
    fn handle(&mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
    ctx: &mut Self::Context) {
        if let ws::Message::Text(data) = msg.expect("Mensaje del cliente invalido") {
            println!("Cliente: {}", data);
        }
        ctx.text("perra".to_owned());
        //ctx.write_raw(ws::Message::Text("perra".to_owned()))
    }
}


#[get("/ws/init/{username}")] //todo:agregar path de username
async fn serve_ws(req: HttpRequest,
    stream: web::Payload,
    data: web::Data<Addr<Feed>>,
    username: web::Path<(String,)>) -> Result<HttpResponse, Error> {
    let sock = ChatSock::new(username.into_inner().0, data.get_ref().clone());
    ws::start(sock, &req, stream)
}

#[derive(Deserialize)]
struct FormUser {
    pub user: String,
}

pub struct IdChecker {
    pub id: String,
    pub addr_feed: Addr<Feed>,
    pub used: bool,
}

impl IdChecker {
    pub fn new(id: String, addr_feed: Addr<Feed>) -> Self {
        println!("created id checker with id: {}", id);
        Self {
            id,
            addr_feed,
            used: false
        }
    }
}

impl Actor for IdChecker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("hellow");
        self.addr_feed.send(chat::CheckId{id:self.id.clone()})
            .into_actor(self)
            .then(|res, s, ctx| {
                println!("bool: {:?}", res);
                match res {
                    Ok(_res) => {
                        s.used = _res;
                    },
                    Err(err) => {
                        println!("BAD at idchecker");
                    }
                }
                fut::ready(())
            }).wait(ctx)
    }
}

#[post("/enter_chat")] //serves the chat.html page only
async fn enter_chat(req: HttpRequest,
    form: web::Form<FormUser>, data: web::Data<Addr<Feed>>)
    -> HttpResponse {
    
    let username = form.into_inner().user;

    let res = data.get_ref().send(CheckId{id: username.clone()}).await;
    match res {
        Ok(res) => {
            if res {
                HttpResponse::build("400".parse().expect("Bad StatusCode"))
                    .body("Invalid username")
            } else {
                let mut hb = Handlebars::new();
                hb.register_template_file("chat", "./public/html/chat.hbs").expect("Failed to register chat user template");
                let body = hb.render("chat", &json!({"username": username})).expect("failed to render chat page");
                HttpResponse::Ok().body(body)
            }
            
        }, Err(err) => {
            panic!(err)
        }
    }
    /*
    if IdChecker::new(username.clone(), data.get_ref().clone()).used {
        println!("ya existe");
        return HttpResponse::build("200".parse().unwrap()).body(""); //lol
    } else {
        println!("ok desde idchekcer");
    }
    */
    
}

struct Contador {
    pub cont: i32
}

impl Contador {
    pub fn new() -> Self {
        Self {
            cont: 0
        }
    }
}

async fn get_counter(data: web::Data<Arc<Mutex<Contador>>>) -> i32 {
    let w = data.lock().unwrap();

    w.cont
}

fn update_contador(data: Arc<Mutex<Contador>>) -> () {
    loop {
        let mut data = data.lock().unwrap();
        thread::sleep_ms(1000);
        println!("{}", data.cont);
        data.cont +=1 ;
    }
}




#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = Arc::new(Mutex::new(Contador::new()));

    let punt = Arc::clone(&data);

    let th = thread::spawn(move || {
        update_contador(data);
    });

    let feed = Feed::new().start();


    HttpServer::new(move || {
        App::new()
            .data(punt.clone())
            .data(feed.clone())
            .service(index)
            .service(saludar)
            .service(pedir_rfc)
            .service(mostrar_rfc)
            .service(serve_css)
            .service(serve_js)
            .service(serve_ws)
            .service(enter_chat)
    })
    .bind((IP, PORT))?
    .listen(TcpListener::bind("192.168.1.64:8080")?)?
    .run()
    .await

}